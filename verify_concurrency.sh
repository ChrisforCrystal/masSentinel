#!/bin/bash

# 获取 Pod 名称
POD=$(kubectl get pod -l app=mas-sentinel-demo-ebpf -o jsonpath="{.items[0].metadata.name}")

if [ -z "$POD" ]; then
  echo "Error: Pod not found"
  exit 1
fi

echo "Target Pod: $POD (demo-app container)"
echo "Sending 20 concurrent requests to 1.1.1.1..."

# 启动 20 个并发请求
for i in {1..20}; do
   kubectl exec "$POD" -c demo-app -- curl -s -o /dev/null -w "Req $i: %{http_code}\n" http://1.1.1.1 &
done

# 等待所有后台任务完成
wait

echo "Verification Complete."
