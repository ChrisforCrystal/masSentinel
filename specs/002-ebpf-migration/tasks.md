# Task Checklist: eBPF Migration

- [x] **Phase 0: Research & Setup**
    - [x] Create Rust eBPF project structure (`mas-sentinel-sidecar/src/bpf`, `mas-sentinel-sidecar/ebpf`) <!-- id: 100 -->
    - [x] Add `aya` and `aya-bpf` dependencies <!-- id: 101 -->
    - [/] Verify local eBPF development environment (vmlinux generation, bpf-linker) <!-- id: 102 -->

- [x] **Phase 1: eBPF Kernel Program**
    - [x] Implement `connect4` program to intercept TCP connections <!-- id: 200 -->
    - [x] Implement helper to filter out Loopback destination (except 15001) <!-- id: 201 -->
    - [x] Implement helper to filter out Sidecar's own UID traffic (prevent loops) <!-- id: 202 -->
    - [x] Create `LRU_HASH` map for storing original destinations <!-- id: 203 -->
    - [x] Implement logic to save `(cookie) -> (original_dst_ip, original_dst_port)` to map <!-- id: 204 -->
    - [x] Implement logic to rewrite destination to `127.0.0.1:15001` <!-- id: 205 -->

- [ ] **Phase 2: Sidecar User-space Logic**
    - [x] Implement `EBPF_Loader` module in Sidecar (load & attach programs) <!-- id: 300 -->
    - [x] Implement `Map_Reader` to query original destination by socket cookie <!-- id: 301 -->
    - [ ] Update `proxy/outbound.rs` to use `Map_Reader` instead of `SO_ORIGINAL_DST` <!-- id: 302 -->
    - [ ] Add fallback mechanism (if map lookup fails, try `SO_ORIGINAL_DST` or passthrough) <!-- id: 303 -->

- [ ] **Phase 3: Integration & Testing**
    - [x] Create `demo-app-ebpf.yaml` for eBPF deployment <!-- id: 400 -->
    - [ ] Build Docker image including eBPF binary <!-- id: 401 -->
    - [ ] Verify traffic interception with `demo-app` <!-- id: 402 -->
    - [ ] Benchmark comparison (iptables vs eBPF) <!-- id: 403 -->
