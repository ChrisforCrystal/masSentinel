use crate::bpf::BpfHandle;
use crate::utils::socket::get_original_dest;

use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

pub async fn start_outbound_proxy(
    port: u16,
    bpf_handle: Option<Arc<Mutex<BpfHandle>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!("Outbound proxy listening on {}", addr);

    loop {
        let (client_socket, _) = listener.accept().await?;
        let bpf_handle_clone = bpf_handle.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_connection(client_socket, bpf_handle_clone).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(
    mut client_socket: TcpStream,
    bpf_handle: Option<Arc<Mutex<BpfHandle>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let peer_addr = client_socket.peer_addr()?;
    tracing::info!("Accepted new outbound connection from {}", peer_addr);

    // 1. Get original destination
    let target_addr = if let Some(handle_mutex) = bpf_handle {
        // eBPF Mode
        // Use Source IP + Source Port as Key to lookup original destination
        let (src_ip, src_port) = match peer_addr {
            std::net::SocketAddr::V4(addr) => (*addr.ip(), addr.port()),
            _ => {
                tracing::warn!("eBPF lookup not supported for IPv6: {}", peer_addr);
                // Fallback or error? For now, fallback to SO_ORIGINAL_DST if possible (but that likely fails too for same reason)
                // Let's just return None from lookup block effectively
                (std::net::Ipv4Addr::new(0, 0, 0, 0), 0)
            }
        };

        if src_port != 0 {
            let lookup_res = {
                let mut handle = handle_mutex.lock().unwrap();
                handle.get_original_dst(src_ip, src_port)
            };

            match lookup_res {
                Some(addr) => {
                    tracing::info!("eBPF Map Hit (Src: {}:{}): -> {}", src_ip, src_port, addr);
                    addr
                }
                None => {
                    tracing::warn!(
                        "eBPF Map Miss for Src {}:{}, falling back to SO_ORIGINAL_DST",
                        src_ip,
                        src_port
                    );
                    // Fallback
                    match get_original_dest(&client_socket) {
                        Ok(addr) => {
                            tracing::info!("SO_ORIGINAL_DST Fallback Hit: -> {}", addr);
                            addr
                        }
                        Err(e) => {
                            // Log error and default?
                            tracing::error!("Fallback failed: {}", e);
                            return Err(e.into());
                        }
                    }
                }
            }
        } else {
            // IPv6 case
            return Err("IPv6 not supported in eBPF mode yet".into());
        }
    } else {
        // Non-BPF Mode
        get_original_dest(&client_socket)?
    };

    tracing::info!("Intercepted connection to {}", target_addr);

    // 1.5 Sentinel Flow Control Check
    let resource = target_addr.ip().to_string();
    if !crate::sentinel::check_flow(&resource) {
        tracing::warn!("Blocked connection to {} by Sentinel", resource);

        // Return HTTP 429 response
        let response = b"HTTP/1.1 429 Too Many Requests\r\nConnection: close\r\nContent-Type: text/plain\r\nContent-Length: 19\r\n\r\nBlocked by Sentinel";
        if let Err(e) = client_socket.write_all(response).await {
            tracing::error!("Failed to write 429 response: {}", e);
        }

        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Blocked by Sentinel",
        )));
    }

    // 2. Connect to upstream
    let mut upstream_socket = TcpStream::connect(target_addr).await?;

    // 3. Bidirectional copy
    let (mut client_reader, mut client_writer) = client_socket.split();
    let (mut upstream_reader, mut upstream_writer) = upstream_socket.split();

    let client_to_upstream = tokio::io::copy(&mut client_reader, &mut upstream_writer);
    let upstream_to_client = tokio::io::copy(&mut upstream_reader, &mut client_writer);

    tokio::try_join!(client_to_upstream, upstream_to_client)?;

    Ok(())
}
