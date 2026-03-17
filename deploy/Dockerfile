FROM ubuntu:24.04 AS builder

# 安装构建依赖
RUN apt update && apt install -y curl build-essential libssl-dev pkg-config chromium-browser
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup install 1.94.0 && rustup default 1.94.0

# 安装Bazel
RUN npm install -g @bazel/bazelisk

# 复制项目文件
WORKDIR /build
COPY . .

# 构建项目
RUN bazel build //:spider-enterprise-crawler

# 运行时镜像
FROM ubuntu:24.04
RUN apt update && apt install -y libssl-dev chromium-browser ca-certificates && rm -rf /var/lib/apt/lists/*

# 复制构建产物
COPY --from=builder /build/bazel-bin/spider-enterprise-crawler /opt/spider/
COPY --from=builder /build/config /opt/spider/config

WORKDIR /opt/spider
ENTRYPOINT ["/opt/spider/spider-enterprise-crawler"]
CMD ["--config", "/opt/spider/config/crawler.yaml"]
