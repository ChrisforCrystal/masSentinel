use aya::maps::HashMap;
use aya::programs::links::CgroupAttachMode;
use aya::programs::CgroupSockAddr;
use aya::Ebpf; // Renamed from Bpf
use aya::Pod;
use std::fs::File;
use std::os::unix::fs::MetadataExt; // Added for MetadataExt
use std::path::Path;
use tracing::{info, warn};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OriginalDst {
    pub ipv4_addr: u32,
    pub port: u16,
}

unsafe impl Pod for OriginalDst {}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SrcKey {
    pub ipv4_addr: u32,
    pub port: u16,
}

unsafe impl Pod for SrcKey {}

pub struct BpfHandle {
    ebpf: Ebpf,
}

impl BpfHandle {
    // 获取原始目标地址的方法
    // Sidecar 收到一个从 eBPF 重定向过来的连接时，会调用这个方法。
    // 1. 获取连接的 源IP 和 源端口 (作为唯一 Key)。
    // 2. 去 eBPF Map (CONNECT_ORIG_DST) 中查询对应的原始 IP 和端口。
    pub fn get_original_dst(
        &mut self,
        src_ip: std::net::Ipv4Addr,
        src_port: u16,
    ) -> Option<std::net::SocketAddr> {
        use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

        let map: HashMap<_, SrcKey, OriginalDst> =
            HashMap::try_from(self.ebpf.map_mut("CONNECT_ORIG_DST")?).ok()?;

        // 构造 Key (注意字节序一致性)
        // 内核中 sock_ops 存储的时候:
        // IP: Network Byte Order (Big Endian)
        // Port: Network Byte Order (Big Endian)
        //
        // 这里 userspace 传入的 IP 已经是 u32, port 是 u16.
        // Rust Ipv4Addr to u32 is usually Host Order? No, it's octets.
        // u32::from(ipv4) returns Host Order (Big Endian on BE machine, LE on LE machine).
        // network is Big Endian.
        // Port: src_port is u16 host order.
        let key = SrcKey {
            ipv4_addr: u32::from(src_ip).to_be(),
            port: src_port.to_be(),
        };

        let origin = map.get(&key, 0).ok()?;

        // 数据转换: eBPF Map 里存的是网络字节序 (Big Endian) 的 u32 和 u16。
        // 我们需要转换成本机字节序，然后构造成 Rust 的 SocketAddr 对象。
        let ip = Ipv4Addr::from(u32::from_be(origin.ipv4_addr));
        let port = u16::from_be(origin.port);

        Some(SocketAddr::V4(SocketAddrV4::new(ip, port)))
    }
}

pub fn load_ebpf() -> Result<Option<BpfHandle>, Box<dyn std::error::Error>> {
    // Path to the compiled eBPF program
    let ebpf_path = "/app/mas-sentinel-ebpf";
    let dev_ebpf_path = "ebpf/target/bpfel-unknown-none/release/mas-sentinel-ebpf";

    let path = if Path::new(ebpf_path).exists() {
        ebpf_path
    } else if Path::new(dev_ebpf_path).exists() {
        dev_ebpf_path
    } else {
        warn!(
            "eBPF binary not found. Skipping eBPF load. (This is expected in non-eBPF environments)"
        );
        return Ok(None);
    };

    info!("Loading eBPF program from {}", path);
    // Use Ebpf::load_file (convenience) or read + load
    let mut ebpf = Ebpf::load_file(path)?;

    // Log buffer if needed (using aya-log)
    // Log buffer if needed (using aya-log)
    // if let Err(e) = aya_log::EbpfLogger::init(&mut ebpf) {
    //     warn!("Failed to init eBPF logger: {}", e);
    // }
    // Populate IGNORED_PORTS map
    // 此处配置要忽略的端口白名单
    let ignored_ports: [u16; 5] = [
        6443,  // K8s API
        10250, // Kubelet
        53,    // DNS
        22,    // SSH
        8080,  // User requested / Health checks
    ];

    // [关键步骤 4] 初始化白名单 Map
    // 我们在用户态将需要忽略的端口写入 eBPF Map。
    if let Some(map) = ebpf.map_mut("IGNORED_PORTS") {
        // 将通用的 BPF Map 包装成 HashMap，方便操作
        let mut ignored_map: HashMap<_, u16, u8> = HashMap::try_from(map)?;

        for port in ignored_ports {
            // [重要] 字节序转换
            // eBPF 程序在内核中看到的是网络字节序 (Big Endian) 的端口号。
            // 而我们的机器通常是 Little Endian。
            // 为了能让内核直接比较，我们必须先把端口号转换为 Big Endian 再写入 Map。
            let port_be = port.to_be();

            // 写入 Map: Key=端口(BE), Value=1 (占位符)
            if let Err(e) = ignored_map.insert(port_be, 1, 0) {
                warn!("Failed to insert ignored port {}: {}", port, e);
            }
        }
        info!("Populated IGNORED_PORTS map with: {:?}", ignored_ports);
    } else {
        warn!("IGNORED_PORTS map not found in eBPF program");
    }

    // [关键步骤 4.5] 填充 IGNORED_CGROUPS (Cgroup ID 白名单)
    // 目的：将 Sidecar 自己的 Cgroup ID 告诉内核，让 eBPF 放行 Sidecar 的流量。
    if let Some(map) = ebpf.map_mut("IGNORED_CGROUPS") {
        let mut ignored_cgroups: HashMap<_, u64, u8> = HashMap::try_from(map)?;
        match get_self_cgroup_id() {
            Ok(cgroup_id) => {
                info!("Detected Self Cgroup ID: {}", cgroup_id);
                if let Err(e) = ignored_cgroups.insert(cgroup_id, 1, 0) {
                    warn!("Failed to insert Cgroup ID {}: {}", cgroup_id, e);
                } else {
                    info!("Successfully whitelisted Self Cgroup ID: {}", cgroup_id);
                }
            }
            Err(e) => {
                warn!(
                    "Failed to get Self Cgroup ID: {}. Sidecar traffic might be intercepted!",
                    e
                );
            }
        }
    } else {
        warn!("IGNORED_CGROUPS map not found in eBPF program");
    }

    // [关键步骤 5] 挂载 eBPF 程序
    // 自动检测当前容器的 Cgroup 路径，实现 Sidecar 隔离。
    // 我们需要读取 /proc/self/cgroup 找到类似 0::/kubepods/... 的路径
    // 然后拼接到 /sys/fs/cgroup 下。
    let cgroup_path = detect_cgroup_path("/sys/fs/cgroup")?;
    info!("Detected cgroup path: {}", cgroup_path);

    let cgroup_file = File::open(&cgroup_path).map_err(|e| {
        warn!("Failed to open cgroup {}: {}", cgroup_path, e);
        e
    })?;

    // [关键步骤 5] 加载程序到内核

    // 1. 加载并挂载 connect4
    {
        // 获取 connect4 程序引用
        let program: &mut CgroupSockAddr = ebpf.program_mut("connect4").unwrap().try_into()?;
        program.load()?;

        // 挂载
        match program.attach(cgroup_file.try_clone()?, CgroupAttachMode::Single) {
            Ok(_) => info!("eBPF connect4 program attached to {}", cgroup_path),
            Err(e) => {
                warn!("Failed to attach eBPF program: {}", e);
            }
        }
    }

    // 2. 加载并挂载 sock_ops
    {
        // 显式指定类型
        use aya::programs::SockOps;
        let sockops_program: &mut SockOps =
            ebpf.program_mut("handle_sockops").unwrap().try_into()?;
        sockops_program.load()?;

        // Attach sock_ops program
        match sockops_program.attach(cgroup_file, CgroupAttachMode::Single) {
            Ok(_) => info!("eBPF sockops program attached to {}", cgroup_path),
            Err(e) => {
                warn!("Failed to attach sock_ops program: {}", e);
            }
        }
    }

    Ok(Some(BpfHandle { ebpf }))
}

// 自动检测当前容器的 Cgroup 路径
//
// 原理：
// 1. 读取 /proc/self/cgroup：这是 Linux 内核提供的接口，记录了当前进程所属的资源组信息。
//    内容示例 (Cgroup v2): 0::/kubelet.slice/kubelet-kubepods.slice/.../cri-containerd-[ID].scope
//
// 2. 解析与拼接：
//    我们需要把上面的相对路径（例如 /kubelet.slice/...）提取出来。
//    然后拼接到宿主机的 Cgroup 挂载点（/sys/fs/cgroup）后面。
//
// 3. 为什么这样做？
//    eBPF 程序需要挂载到某个具体的 Cgroup 节点上。
//    如果我们挂载到根 (/sys/fs/cgroup)，就会拦截整个宿主机的流量（这是之前的 bug）。
//    通过这种“自我反省”的方式，Sidecar 能精准找到自己头顶上的 Cgroup，从而只拦截自己的 Pod 流量。
fn detect_cgroup_path(base_path: &str) -> Result<String, anyhow::Error> {
    use std::io::{BufRead, BufReader};
    use std::path::Path;

    // 读取 /proc/self/cgroup
    let file = File::open("/proc/self/cgroup")?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        // 格式通常是: 0::/kubepods/.../pod[UID].slice/cri-containerd-[ID].scope
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() == 3 {
            let path = parts[2];
            if !path.is_empty() && path != "/" {
                // [Fix] 之前直接使用了当前容器的路径，导致只拦截了 Sidecar 自己的流量。
                // 实际上，Demo App 运行在同一个 Pod 的另一个容器中，它们有不同的 Cgroup Scope，
                // 但共享同一个父级 Pod Cgroup (Slice)。
                //
                // 我们需要挂载到父级 Pod Slice 上，才能拦截整个 Pod (包括应用容器) 的流量。
                // 逻辑：去除路径的最后一段 (Container Scope)，保留父目录 (Pod Slice)。
                let full_path = format!("{}{}", base_path, path);
                let path_obj = Path::new(&full_path);

                // 尝试获取父目录
                if let Some(parent) = path_obj.parent() {
                    let parent_str = parent.to_string_lossy().to_string();
                    info!("Detected Pod Cgroup (Parent): {}", parent_str);
                    return Ok(parent_str);
                }

                // 如果无法获取父目录（极少见），回退到当前目录
                warn!(
                    "Could not find parent cgroup, using container cgroup: {}",
                    full_path
                );
                return Ok(full_path);
            }
        }
    }

    Ok(base_path.to_string())
}

// 获取当进程的 Cgroup ID (Inode Number)
// 用于填入 eBPF 白名单
fn get_self_cgroup_id() -> Result<u64, anyhow::Error> {
    // 1. 读取 /proc/self/cgroup 获取路径
    // 注意: 这里我们需要的是 Container 级别的 Cgroup ID，不是 Pod 级别的。
    // 因为 Sidecar 进程运行在 Container Cgroup 中。
    let file = std::fs::File::open("/proc/self/cgroup")?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;

    let mut cgroup_path = String::new();
    for line in reader.lines() {
        let line = line?;
        // 0::/kubepods/.../cri-containerd-...scope
        let parts: Vec<&str> = line.splitn(3, ':').collect();
        if parts.len() == 3 {
            let path = parts[2];
            if !path.is_empty() {
                cgroup_path = format!("/sys/fs/cgroup{}", path);
                break;
            }
        }
    }

    if cgroup_path.is_empty() {
        return Err(anyhow::anyhow!(
            "Could not find cgroup path in /proc/self/cgroup"
        ));
    }

    // 2. 获取目录的元数据 (Metadata)
    // Inode 号就是 Cgroup ID
    let meta = std::fs::metadata(&cgroup_path)
        .map_err(|e| anyhow::anyhow!("Failed to read metadata for {}: {}", cgroup_path, e))?;

    Ok(meta.ino())
}
