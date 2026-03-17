#!/bin/bash
set -e

echo "===== 开始构建spider-rs企业级爬虫 ====="

# 加载环境变量
if [ -f .env ]; then
    source .env
fi

# 构建二进制文件
bazel build //:spider-enterprise-crawler

# 构建分发包
bazel build //:crawler-package

echo "===== 构建完成 ====="
echo "二进制路径: $(bazel cquery //:spider-enterprise-crawler --output=files)"
echo "分发包路径: $(bazel cquery //:crawler-package --output=files)"
