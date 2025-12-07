#!/bin/bash
# verify_limit.sh
# 验证 Sentinel 限流规则 (1.1.1.1 限流 1 QPS)

echo "Starting Rate Limit Verification..."
POD_NAME=$(kubectl get pod -l app=mas-sentinel-demo-ebpf -o jsonpath="{.items[0].metadata.name}")

if [ -z "$POD_NAME" ]; then
    echo "Error: Demo pod not found."
    exit 1
fi

echo "Target Pod: $POD_NAME (demo-app container)"

# 直接在 demo-app 容器(curlimages/curl)中执行 curl
# 期望：第1个请求成功(200)，后续请求被限流(429)
kubectl exec -it $POD_NAME -c demo-app -- /bin/sh -c '
    echo "Sending 5 requests to 1.1.1.1..."
    echo "--------------------------------"
    for i in $(seq 1 20); do
        echo -n "Request $i: "
        # -I: 只取 Header
        # -s: 静默模式
        # -w: 输出 HTTP 状态码
        # --connect-timeout: 超时设置
        curl -I -s -o /dev/null -w "%{http_code}\n" --connect-timeout 2 http://1.1.1.1/ || echo "Failed"
    done
'
