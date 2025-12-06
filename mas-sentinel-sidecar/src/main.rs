mod proxy;
mod utils;

mod config;
mod sentinel;

use proxy::outbound::start_outbound_proxy;
use sentinel::init_sentinel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    tracing::info!("Sentinel-Rust Sidecar starting...");

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
    start_outbound_proxy(15001).await?;

    Ok(())
}
