use crate::utils::socket::get_original_dest;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream}; // Import for write_all

pub async fn start_outbound_proxy(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!("Outbound proxy listening on {}", addr);

    loop {
        let (client_socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(e) = handle_connection(client_socket).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(mut client_socket: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Get original destination
    // On non-Linux (e.g. macOS dev), this will fail.
    // For dev convenience, we might want to fallback to peer_addr or a fixed target if env var set?
    // But strictly following spec:
    let target_addr = match get_original_dest(&client_socket) {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to get original destination: {}", e);
            tracing::error!("Failed to get original destination: {}", e);
            // For testing on Mac, let's allow a fallback if explicitly enabled via env?
            // Or just fail. Let's fail for now to be safe.
            return Err(Box::new(e));
        }
    };

    tracing::info!("Intercepted connection to {}", target_addr);

    // 1.5 Sentinel Flow Control Check
    // We use the target IP (or domain if we could resolve it) as the resource.
    // For now, we use the IP string.
    let resource = target_addr.ip().to_string();
    if !crate::sentinel::check_flow(&resource) {
        tracing::warn!("Blocked connection to {} by Sentinel", resource);

        // Return HTTP 429 response
        let response = b"HTTP/1.1 429 Too Many Requests\r\nConnection: close\r\nContent-Type: text/plain\r\nContent-Length: 19\r\n\r\nBlocked by Sentinel";
        if let Err(e) = client_socket.write_all(response).await {
            tracing::error!("Failed to write 429 response: {}", e);
        }

        // Terminate connection (by returning error, which closes the socket)
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Blocked by Sentinel",
        )));
    }

    // 2. Connect to upstream (连接上游真实目标)
    // 此时 Sidecar 作为客户端，发起连接到原本的目标地址 (例如 google.com:80)
    let mut upstream_socket = TcpStream::connect(target_addr).await?;

    // 3. Bidirectional copy (双向数据拷贝)
    // 将 TCP 流分离为“读半部”和“写半部”，以便同时处理读写
    let (mut client_reader, mut client_writer) = client_socket.split();
    let (mut upstream_reader, mut upstream_writer) = upstream_socket.split();

    // 创建两个异步任务：
    // 任务 A: 把客户端发来的数据 (client_reader) 拷贝给上游 (upstream_writer) -> 请求流
    let client_to_upstream = tokio::io::copy(&mut client_reader, &mut upstream_writer);

    // 任务 B: 把上游返回的数据 (upstream_reader) 拷贝给客户端 (client_writer) -> 响应流
    let upstream_to_client = tokio::io::copy(&mut upstream_reader, &mut client_writer);

    // 等待两个任务完成。只要有一个方向断开或出错，整个连接就结束。
    tokio::try_join!(client_to_upstream, upstream_to_client)?;

    Ok(())
}
