use std::io;
use tokio::net::TcpStream;

#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "linux")]
pub fn get_original_dest(stream: &TcpStream) -> io::Result<std::net::SocketAddr> {
    use std::mem;
    use std::net::SocketAddrV4;

    let fd = stream.as_raw_fd();
    unsafe {
        let mut addr: libc::sockaddr_in = mem::zeroed();
        let mut len = mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;

        let ret = libc::getsockopt(
            fd,
            libc::SOL_IP,
            libc::SO_ORIGINAL_DST,
            &mut addr as *mut _ as *mut libc::c_void,
            &mut len,
        );

        if ret != 0 {
            return Err(io::Error::last_os_error());
        }

        let ip = std::net::Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr));
        let port = u16::from_be(addr.sin_port);
        Ok(std::net::SocketAddr::V4(SocketAddrV4::new(ip, port)))
    }
}

#[cfg(not(target_os = "linux"))]
pub fn get_original_dest(_stream: &TcpStream) -> io::Result<std::net::SocketAddr> {
    // For local development on non-Linux, we can't get SO_ORIGINAL_DST.
    // We'll return an error or a dummy address if needed for testing logic flow.
    // Spec requires SO_ORIGINAL_DST, so error is appropriate.
    Err(io::Error::new(
        io::ErrorKind::Other,
        "SO_ORIGINAL_DST only supported on Linux",
    ))
}
