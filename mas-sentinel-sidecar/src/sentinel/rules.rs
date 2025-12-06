use sentinel_core::flow;

pub fn load_hardcoded_rules() {
    // 加载一个硬编码的测试规则：限制 1.1.1.1 的 QPS 为 1
    let rule = flow::Rule {
        // resource: 资源名称。我们在 proxy 中把目标 IP 当作资源名。
        resource: "1.1.1.1".into(),

        // threshold: 阈值。这里设置为 1.0，表示每秒只允许 1 个请求。
        threshold: 1.0,

        // calculate_strategy: 统计/计算策略 (如何计算流量)
        // CalculateStrategy::Direct -> 直接计数。这是最常用的模式，直接统计当前时间窗口内的请求数。
        // (其他可能的值：WarmUp 预热模式，用于防止系统刚启动时流量突增)
        calculate_strategy: flow::CalculateStrategy::Direct,

        // control_strategy: 控制/流控策略 (超限了怎么办)
        // ControlStrategy::Reject -> 直接拒绝。check_flow 会返回 false，我们直接断开连接。
        // (其他可能的值：Throttling 匀速排队，让请求平滑通过，适合削峰填谷)
        control_strategy: flow::ControlStrategy::Reject,

        ..Default::default()
    };

    // Sentinel 内部使用 Arc (原子引用计数) 来管理规则，所以需要包一层 Arc
    flow::load_rules(vec![std::sync::Arc::new(rule)]);
    println!("Loaded hardcoded Sentinel rules: Limit 1.1.1.1 to 1 QPS");
}

pub fn load_rules_from_proto(proto_rules: Vec<crate::config::configv1::FlowRule>) {
    let mut rules = Vec::new();
    for pr in proto_rules {
        let rule = flow::Rule {
            resource: pr.resource,
            threshold: pr.count,
            // 目前 MVP 版本我们把策略写死为 Direct + Reject
            // 未来可以从 proto 中读取这些配置，支持更多样的流控效果
            calculate_strategy: flow::CalculateStrategy::Direct,
            control_strategy: flow::ControlStrategy::Reject,
            ..Default::default()
        };
        rules.push(std::sync::Arc::new(rule));
    }
    flow::load_rules(rules);
    println!("Updated Sentinel rules from Control Plane");
}
