# Tasks: Sentinel-Rust Sidecar (Mesh Lite)

**Feature**: `001-sentinel-rust-sidecar`
**Status**: Planning

## Phase 1: Setup (Project Initialization)

- [x] T001 Create Rust workspace and Sidecar crate structure in `mas-sentinel-sidecar/Cargo.toml`
- [x] T002 [P] Initialize Go Control Plane module in `mas-sentinel-controller/go.mod`
- [x] T003 [P] Create Kubernetes deployment manifests directory `deploy/`
- [x] T004 Define gRPC protocol in `mas-sentinel-sidecar/proto/config.proto` (copy from contracts)

## Phase 2: Foundational (Blocking Prerequisites)

- [x] T005 Implement `init-iptables.sh` logic (embedded in `deploy/demo-app.yaml`)
- [x] T006 [P] Implement gRPC code generation for Rust in `mas-sentinel-sidecar/build.rs`
- [x] T007 [P] Implement gRPC code generation for Go in `mas-sentinel-controller/Makefile`

## Phase 3: User Story 1 - Transparent Traffic Interception (P1)

**Goal**: Intercept outbound TCP traffic and redirect to Sidecar without loops.
**Independent Test**: Deploy Pod with Sidecar, `curl` external service, verify traffic hits Sidecar.

- [x] T008 [US1] Implement TCP Listener on port 15001 in `mas-sentinel-sidecar/src/proxy/outbound.rs`
- [x] T009 [US1] Implement `SO_ORIGINAL_DST` retrieval logic in `mas-sentinel-sidecar/src/utils/socket.rs`
- [x] T010 [US1] Implement basic TCP Proxy loop (connect to original dst) in `mas-sentinel-sidecar/src/proxy/outbound.rs`
- [x] T011 [US1] Create Dockerfile for Sidecar in `deploy/Dockerfile.sidecar`
- [x] T012 [US1] Create Pod YAML with Init Container and Sidecar in `deploy/demo-app.yaml`
- [x] T013 [US1] Verify interception in local Docker/Kind environment

## Phase 4: User Story 2 - Sentinel Traffic Governance (P1)

**Goal**: Enforce Sentinel flow control rules on intercepted traffic.
**Independent Test**: Configure QPS limit 1, send 10 reqs, verify 9 blocked.

- [x] T014 [US2] Add `sentinel-rs` dependency to `mas-sentinel-sidecar/Cargo.toml`
- [x] T015 [US2] Initialize Sentinel with hardcoded rules for testing in `mas-sentinel-sidecar/src/sentinel/mod.rs`
- [x] T016 [US2] Integrate Sentinel entry check into Proxy loop in `mas-sentinel-sidecar/src/proxy/outbound.rs`
- [x] T017 [US2] Implement connection termination logic for blocked requests in `mas-sentinel-sidecar/src/proxy/outbound.rs`

## Phase 5: User Story 3 - Dynamic Configuration Sync (P2)

**Goal**: Fetch rules from Control Plane dynamically.
**Independent Test**: Update rule in Control Plane, verify Sidecar applies it.

- [x] T018 [US3] Implement `ConfigService` gRPC Server in `mas-sentinel-controller/pkg/server/server.go`
- [x] T019 [US3] Implement `ConfigService` gRPC Client in `mas-sentinel-sidecar/src/config/grpc.rs`
- [x] T020 [US3] Implement dynamic rule loading logic (DataSource) in `mas-sentinel-sidecar/src/sentinel/rules.rs`
- [x] T021 [US3] Integrate gRPC Client with Sentinel DataSource in `mas-sentinel-sidecar/src/main.rs`

## Phase 6: Polish & Cross-Cutting Concerns

- [x] T022 Add structured logging (tracing) to Sidecar in `mas-sentinel-sidecar/src/main.rs`
- [x] T025 Implement HTTP 429 response for blocked requests in `mas-sentinel-sidecar/src/proxy/outbound.rs`
- [x] T023 Add basic metrics (Prometheus) endpoint (optional for MVP)
- [x] T024 Update README.md with build and deployment instructions

## Dependencies

- Phase 1 -> Phase 2 -> Phase 3 -> Phase 4 -> Phase 5 -> Phase 6
- US1 (Interception) is prerequisite for US2 (Governance)
- US2 (Governance) is prerequisite for US3 (Dynamic Config)

## Implementation Strategy

1. **MVP (US1 + US2)**: Focus on getting traffic intercepted and blocked by hardcoded rules. This proves the core value proposition.
2. **Dynamic (US3)**: Once the data plane works, add the control plane integration.
