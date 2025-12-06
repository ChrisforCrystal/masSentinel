# Task Checklist: eBPF Migration

- [ ] **Phase 0: Research & Setup**
    - [ ] Create Rust eBPF project structure (`mas-sentinel-sidecar/src/bpf`, `mas-sentinel-sidecar/ebpf`) <!-- id: 100 -->
    - [ ] Add `aya` and `aya-bpf` dependencies <!-- id: 101 -->
    - [ ] Verify local eBPF development environment (vmlinux generation, bpf-linker) <!-- id: 102 -->

- [ ] **Phase 1: eBPF Kernel Program**
    - [ ] Implement `connect4` program to intercept TCP connections <!-- id: 200 -->
    - [ ] Implement helper to filter out Loopback destination (except 15001) <!-- id: 201 -->
    - [ ] Implement helper to filter out Sidecar's own UID traffic (prevent loops) <!-- id: 202 -->
    - [ ] Create `LRU_HASH` map for storing original destinations <!-- id: 203 -->
    - [ ] Implement logic to save `(cookie) -> (original_dst_ip, original_dst_port)` to map <!-- id: 204 -->
    - [ ] Implement logic to rewrite destination to `127.0.0.1:15001` <!-- id: 205 -->

- [ ] **Phase 2: Sidecar User-space Logic**
    - [ ] Implement `EBPF_Loader` module in Sidecar (load & attach programs) <!-- id: 300 -->
    - [ ] Implement `Map_Reader` to query original destination by socket cookie <!-- id: 301 -->
    - [ ] Update `proxy/outbound.rs` to use `Map_Reader` instead of `SO_ORIGINAL_DST` <!-- id: 302 -->
    - [ ] Add fallback mechanism (if map lookup fails, try `SO_ORIGINAL_DST` or passthrough) <!-- id: 303 -->

- [ ] **Phase 3: Integration & Testing**
    - [ ] Create `init-ebpf` script or integrate into binary startup <!-- id: 400 -->
    - [ ] Update `deploy/sidecar.yaml` with necessary privileges (`CAP_BPF`, etc.) <!-- id: 401 -->
    - [ ] Verify traffic interception with `demo-app` <!-- id: 402 -->
    - [ ] Benchmark comparison (iptables vs eBPF) <!-- id: 403 -->
