#!/bin/bash
set -e

# 加载环境变量
if [ -f .env ]; then
    source .env
fi

# 运行爬虫
bazel run //:spider-enterprise-crawler -- --config config/crawler.yaml
