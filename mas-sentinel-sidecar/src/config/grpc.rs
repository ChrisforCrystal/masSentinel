use crate::config::configv1::config_service_client::ConfigServiceClient;
use crate::config::configv1::WatchRequest;

pub async fn connect_control_plane(addr: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ConfigServiceClient::connect(addr).await?;
    tracing::info!("Connected to Control Plane");

    let request = tonic::Request::new(WatchRequest {
        app_name: "demo-app".into(),
        node_id: "node-1".into(),
    });

    let mut stream = client.watch_config(request).await?.into_inner();

    while let Some(response) = stream.message().await? {
        tracing::info!(
            "Received config update with {} rules",
            response.flow_rules.len()
        );
        crate::sentinel::rules::load_rules_from_proto(response.flow_rules);
    }

    Ok(())
}
