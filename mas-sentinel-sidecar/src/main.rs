mod bpf;
mod config;
mod proxy;
mod sentinel;
mod utils;

use bpf::load_ebpf;
use proxy::outbound::start_outbound_proxy;
use sentinel::init_sentinel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Sentinel-Rust Sidecar starting...");

    // Try to load eBPF (fail-open)
    // Keep handle alive to prevent map unload (unless pinned)
    // load_ebpf returns Result<Option<BpfHandle>, Error>
    let _bpf_handle = match load_ebpf() {
        Ok(maybe_handle) => maybe_handle,
        Err(e) => {
            tracing::warn!("eBPF initialization failed: {}", e);
            None
        }
    };

    // Init Sentinel
    init_sentinel();

    // Start Control Plane Client (Async)
    tokio::spawn(async {
        // Assuming Control Plane is at localhost:50051 for now
        // In K8s, this would be a service DNS
        let addr = "http://127.0.0.1:50051".to_string();
        if let Err(e) = crate::config::grpc::connect_control_plane(addr).await {
            eprintln!("Control Plane connection error: {}", e);
        }
    });

    // Start outbound proxy on 15001
    // Flatten Option<BpfHandle> to Option<Arc<Mutex<BpfHandle>>>
    let bpf_handle_arg = _bpf_handle.map(|h| std::sync::Arc::new(std::sync::Mutex::new(h)));
    start_outbound_proxy(15001, bpf_handle_arg).await?;

    Ok(())
}
