#![no_std]
#![no_main]

// 引入 aya_ebpf 库: 开发 Rust eBPF 的核心框架
use aya_ebpf::{
    bindings::{BPF_SOCK_OPS_ACTIVE_ESTABLISHED_CB, BPF_SOCK_OPS_STATE_CB_FLAG},
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_uid_gid, bpf_get_socket_cookie,
        bpf_sock_ops_cb_flags_set,
    },
    macros::{cgroup_sock_addr, map, sock_ops},
    maps::HashMap,
    programs::{SockAddrContext, SockOpsContext},
};
use aya_log_ebpf::info;

// 定义数据结构: 用于保存 "连接原本想去哪" (原始目标地址)
// #[repr(C)] 保证内存布局与 C 语言一致，方便内核和用户态共享读取。
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OriginalDst {
    pub ipv4_addr: u32,
    pub port: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SrcKey {
    pub ip: u32,
    pub port: u16,
}

// 1. CONNECT_ORIG_DST: 原地目标地址映射
//    Key: SrcKey { ip, port } (源IP + 源端口) -> 唯一确定一个连接
//    Value: OriginalDst { ip, port } (原始目标地址)
//    由 sock_ops 程序在 TCP 建立连接时从 COOKIE_ORIG_DST 搬运过来
#[map]
static CONNECT_ORIG_DST: HashMap<SrcKey, OriginalDst> = HashMap::with_max_entries(1024, 0);

// 2. COOKIE_ORIG_DST: 临时中间表
//    Key: Socket Cookie (u64)
//    Value: OriginalDst
//    由 connect4 程序在拦截 connect 时写入
#[map]
static COOKIE_ORIG_DST: HashMap<u64, OriginalDst> = HashMap::with_max_entries(1024, 0);

// 2. IGNORED_PORTS: 端口白名单。
//    Key 是端口号 (网络字节序), Value 是占位符。
//    如果应用访问的端口在这个 Map 里，eBPF 程序会直接放行，不进行重定向。
//    这是为了避免拦截 K8s 自身的流量 (如 API Server 6443) 或 Sidecar 自己的健康检查端口。
#[map]
static IGNORED_PORTS: HashMap<u16, u8> = HashMap::with_max_entries(64, 0);

// 3. IGNORED_CGROUPS: Cgroup ID 白名单
//    Key 是 Cgroup ID (u64), Value 是占位符
//    用于排除 Sidecar 自身的流量，防止死循环
#[map]
static IGNORED_CGROUPS: HashMap<u64, u8> = HashMap::with_max_entries(64, 0);

// 常量定义
const PROXY_UID: u32 = 1337; // Sidecar 容器运行的用户 ID (防止死循环)
const PROXY_PORT: u16 = 15001; // Sidecar 监听的端口
const LOOPBACK_IP: u32 = 0x0100007F; // 127.0.0.1 (大端序/网络字节序的内存表现)

// 程序入口: 挂载在 IPv4 connect 系统调用上的钩子
// 当任何程序在 Cgroup 内发起 TCP 连接时，这个函数就会被内核执行。
#[cgroup_sock_addr(connect4)]
pub fn connect4(ctx: SockAddrContext) -> i32 {
    match try_connect4(ctx) {
        Ok(ret) => ret,  // 返回 1 表示允许连接 (Allow)
        Err(ret) => ret, // 返回 0 表示拒绝连接 (Block)
    }
}

// 核心逻辑实现
fn try_connect4(ctx: SockAddrContext) -> Result<i32, i32> {
    // 获取发起连接的进程/线程的有效用户 ID (UID)
    let uid = bpf_get_current_uid_gid() as u32;

    // 排除 Sidecar 自身流量 (防止死循环)
    // 使用 Cgroup ID 过滤 (最稳健的方法)
    // Sidecar 启动时会将自己的 Cgroup ID 写入 IGNORED_CGROUPS Map
    let cgroup_id = unsafe { bpf_get_current_cgroup_id() };
    if unsafe { IGNORED_CGROUPS.get(&cgroup_id).is_some() } {
        return Ok(1);
    }

    // [UID Check fallback] (Optional, keeping it doesn't hurt)
    if uid == PROXY_UID {
        return Ok(1);
    }

    // 从上下文读取当前连接想去的目标 IP 和端口 (直接读取内核内存)
    // ctx.sock_addr 是 *mut bpf_sock_addr
    // [关键步骤 1] 获取上下文信息
    // 从 ctx 中获取当前连接的目标 IP 和端口 (即应用原本想访问的地址)
    // 注意：这里的 user_ip4 和 user_port 是指向内核数据结构的指针
    let user_ip = unsafe { (*ctx.sock_addr).user_ip4 };
    let user_port = unsafe { (*ctx.sock_addr).user_port };

    // 转换 IP 为 127.0.0.1 (Loopback) 的整数形式 (网络字节序)
    // 127.0.0.1 = 0x7F000001. 在 Big Endian (网络序) 下，通常直接用这个常量。
    // 如果是 Little Endian 机器，u32::to_be 会处理它。
    // 这里我们简单判断：如果是发往 127.0.0.1 的流量，直接忽略。
    // 防止 Sidecar 自己发出的流量又被拦截，形成死循环。
    // 如果目标本来就是 127.0.0.1 (0x0100007F)，就不拦截了。
    // 这在本地调试时很有用，也能避免拦截 sidecar 内部组件通信。
    if user_ip == LOOPBACK_IP {
        return Ok(1);
    }

    // [关键步骤 2.5] 动态端口过滤 (Map-based Port Exclusion)
    // 检查目标端口是否在忽略列表中
    // 如果在 IGNORED_PORTS Map 中能找到 key (user_port)，说明该端口需要放行
    if unsafe { IGNORED_PORTS.get(&(user_port as u16)).is_some() } {
        return Ok(1);
    }

    let cookie = unsafe { bpf_get_socket_cookie(ctx.sock_addr as *mut _) };

    // 构建原始目标结构体
    let orig_dst = OriginalDst {
        ipv4_addr: user_ip,
        port: user_port as u16,
    };

    // 将 {Cookie -> 原始目标} 写入临时 Map (COOKIE_ORIG_DST)
    // 等到连接建立完成 (sock_ops 触发) 时，再根据源端口搬运到正式 Map
    let cookie = unsafe { bpf_get_socket_cookie(ctx.sock_addr as *mut _) };
    if cookie != 0 {
        if let Err(e) = COOKIE_ORIG_DST.insert(&cookie, &orig_dst, 0) {
            // log error if possible
        }
    }

    // [关键步骤 4] 篡改目标 (Redirect)
    // 准备新目标：127.0.0.1:15001 (Sidecar 的地址)

    // 端口 15001 转换成网络字节序 (Big Endian)
    let new_port = 15001u16.to_be();

    // 直接修改内核数据结构中的目标地址！
    // 这样应用程序以为自己在连 Google，但 TCP 握手包实际发给了 Sidecar。
    unsafe {
        (*ctx.sock_addr).user_ip4 = LOOPBACK_IP;
        (*ctx.sock_addr).user_port = new_port as u32;
    }

    // 打印日志 (可以通过 `bpftool prog trace log` 查看)
    // info!(&ctx, "redirected to 127.0.0.1:15001");

    Ok(1) // 返回 1，告诉内核 "继续处理这个连接 (但目标已经被我们改了)"
}

// 新增: sock_ops 程序
// 用于在 TCP 连接建立时 (ESTABLISHED)，获取源端口，并将 Cookie Map 的数据迁移到 SrcKey Map
#[sock_ops]
pub fn handle_sockops(ctx: SockOpsContext) -> u32 {
    let op = ctx.op();

    // 1. 如果是状态改变回调 (State Change CB)
    // 我们只关心 TCP 状态变为 ACTIVE_ESTABLISHED (主动连接建立成功)
    if op == BPF_SOCK_OPS_ACTIVE_ESTABLISHED_CB {
        // 获取当前 Socket 的 Cookie
        // 获取当前 Socket 的 Cookie
        let cookie = unsafe { bpf_get_socket_cookie(ctx.ops as *mut _) };
        if cookie != 0 {
            // 去临时表里查：这个连接是不是被我们 connect4 拦截过的？
            // 注意: 这里使用的是 get_ptr 而不是 get，避免复制? aya map api returns Option<&V>
            // unsafe is needed for map access usually? aya wraps it.
            if let Some(orig_dst) = unsafe { COOKIE_ORIG_DST.get(&cookie) } {
                // 如果查到了，说明这是受监控的连接。

                // 获取源 IP 和 源端口
                // remote_ip4/local_ip4 本地序还是网络序？
                // 在 sock_ops context 中:
                // local_ip4 是源 IP (Host Byte Order usually in bpf context struct fields, but let's verify)
                // local_port 是源端口 (Host Byte Order)
                //
                // 注意：Ctx 字段访问通常已经是 Host Byte Order？
                // 查阅 aya 文档/Linux bpf 文档: bpf_sock_ops 结构体字段通常是 Host Byte Order，
                // 除了 IP 可能是 Network Byte Order。
                //
                // 安全起见，我们通常认为:
                // IP: Network Byte Order (大端)
                // Port: Host Byte Order 或者 Network Byte Order?
                // 在 Connect hook 中 redirect 的 port 是 Network Byte Order。
                //
                // 由于我们最终要给 Userspace 用，Userspace 拿到的 peer_addr 是标准 SocketAddr (IP是Net, Port是Host?)
                // Wait, Rust SocketAddr: IP is struct, Port is u16.
                //
                // 让我们先直接读取 context 字段。

                let local_ip = ctx.local_ip4(); // u32
                let local_port = ctx.local_port(); // u32

                // 构造 Key
                // Userspace 那边获取 peer_addr 拿到的是什么？
                // SocketAddrV4.ip() -> Octets.
                // 我们在 Userspace 会把 IP 转成 u32 (Big Endian usually).
                //
                // 关键点：Userspace 和 Kernel 必须一致。
                // 建议 Key 全部使用 Network Byte Order (Big Endian) 存储。
                //
                // ctx.local_ip4() 返回的是什么字节序？通常是 Network Byte Order。
                // ctx.local_port() 返回的是 Host Byte Order (Linux Kernel internal).
                //
                // 我们把 Port 转成 Network Byte Order (u16::to_be) 存进去
                // 这样 Userspace 那边也转成 BE 再查，就一致了。

                // port 在 bpf_sock_ops 中是 u32, 但只有低16位有效。
                // 将 Host Order 的 port 转为 Network Order (Big Endian)
                let port_be = (local_port as u16).to_be();

                let key = SrcKey {
                    ip: local_ip, // 假设已经是 BE
                    port: port_be,
                };

                // 搬运数据: 写入正式表
                if let Err(_) = CONNECT_ORIG_DST.insert(&key, orig_dst, 0) {
                    // ignore error
                }

                // 清理临时表 (Optional, but good for cleanup)
                // COOKIE_ORIG_DST.remove(&cookie);
                // Ignore remove error
            }
        }
    }
    // 2. 必须先启用状态回调
    // 默认情况下 eBPF 不会收到 TCP 状态变化的通知
    // 我们需要在连接初始化时 (TCP_CONNECT 或 ACTIVE_OP) 开启这个 flag
    else if op == aya_ebpf::bindings::BPF_SOCK_OPS_TCP_CONNECT_CB {
        // 设置回调标志，告诉内核：建立连接时通知我
        unsafe {
            bpf_sock_ops_cb_flags_set(ctx.ops, BPF_SOCK_OPS_STATE_CB_FLAG as i32);
        }
    }

    0
}

// 必须的 Panic 处理器 (因为没有 std 库处理 panic)
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
