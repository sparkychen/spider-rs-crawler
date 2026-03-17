#!/bin/bash
set -euo pipefail

# ===================== 核心配置区（适配当前项目）=====================
# Bazel 构建目标（与项目BUILD文件保持一致）
BAZEL_BIN_TARGET="//:spider-enterprise-crawler"
BAZEL_BUNDLE_TARGET="//:crawler-package"
# Docker 配置（匹配deploy/k8s/cronjob.yaml & docker-compose.yml）
DOCKER_REGISTRY="your-registry"
IMAGE_NAME="spider-enterprise-crawler"
IMAGE_TAG="v2.47.22"  # 匹配Cargo.toml版本
# K8s 配置（匹配deploy/k8s/下的配置文件）
K8S_NAMESPACE="spider-crawler"
K8S_CONFIGMAP="spider-crawler-config"
K8S_SECRET="spider-secrets"
K8S_CRONJOB="spider-enterprise-crawler"
# 构建模式（默认debug，可通过--release覆盖）
BUILD_MODE="debug"

# ===================== 工具函数 =====================
# 格式化日志输出
log_info() {
  echo -e "\033[32m[INFO] $(date +%Y-%m-%d\ %H:%M:%S) $1\033[0m"
}

log_warn() {
  echo -e "\033[33m[WARN] $(date +%Y-%m-%d\ %H:%M:%S) $1\033[0m"
}

log_error() {
  echo -e "\033[31m[ERROR] $(date +%Y-%m-%d\ %H:%M:%S) $1\033[0m"
  exit 1
}

# 环境依赖校验（适配当前项目依赖）
check_dependencies() {
  log_info "开始校验依赖环境..."
  
  # 基础工具校验
  required_tools=("bazel" "rustup" "cargo" "docker" "kubectl" "xvfb-run" "docker-compose")
  for tool in "${required_tools[@]}"; do
    if ! command -v "$tool" &> /dev/null; then
      if [[ "$tool" == "xvfb-run" ]]; then
        log_warn "xvfb未安装，将自动尝试安装（用于无界面运行Chrome）"
        sudo apt update && sudo apt install -y xvfb
      elif [[ "$tool" == "kubectl" && "$ENABLE_DEPLOY" != "true" ]]; then
        log_warn "kubectl未安装，仅影响K8s部署功能"
      elif [[ "$tool" == "docker-compose" && "$ENABLE_DOCKER" != "true" ]]; then
        log_warn "docker-compose未安装，仅影响容器化部署功能"
      else
        log_error "核心工具 $tool 未安装，请参考部署文档安装"
      fi
    fi
  done

  # Rust环境校验
  if ! rustup show | grep -q "stable"; then
    log_error "未检测到Stable Rust环境，请执行：curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  fi

  # 环境变量校验（敏感配置）
  if [[ -f .env ]]; then
    source .env
    required_envs=("LOGIN_USERNAME" "LOGIN_PASSWORD" "MINIO_ACCESS_KEY" "MINIO_SECRET_KEY")
    for env in "${required_envs[@]}"; do
      if [[ -z "${!env:-}" ]]; then
        log_warn "环境变量 $env 未配置，将导致登录/MinIO存储失败"
      fi
    done
  else
    log_error "未找到 .env 文件，请先在项目根目录创建并配置敏感信息"
  fi

  # 配置文件校验
  if [[ ! -f "config/crawler.yaml" ]]; then
    log_error "未找到爬虫配置文件 config/crawler.yaml，请确认配置文件存在"
  fi

  log_info "依赖环境校验完成"
}

# 清理旧构建产物（适配项目目录结构）
clean_old_build() {
  log_info "清理旧构建产物..."
  # Bazel缓存清理
  bazel clean --expunge
  # Rust产物清理
  rm -rf target/ Cargo.lock.bak *.rs.bk
  # 打包/分发目录清理
  rm -rf dist/ downloads/ crawl_results/
  # 日志/临时文件清理
  rm -rf logs/ *.log *.tmp *.temp
  # 重建必要目录
  mkdir -p dist downloads logs
  log_info "旧产物清理完成"
}

# Bazel构建二进制（适配项目构建规则）
build_binary() {
  log_info "开始构建${BUILD_MODE}模式二进制..."
  local bazel_build_flags=()
  if [[ "$BUILD_MODE" == "release" ]]; then
    bazel_build_flags+=("--compilation_mode=opt")  # 优化编译（Release）
  fi
  # 执行Bazel构建
  bazel build "${bazel_build_flags[@]}" "$BAZEL_BIN_TARGET"
  
  # 复制二进制到dist目录（便于分发）
  local bin_path=$(bazel cquery "$BAZEL_BIN_TARGET" --output=files)
  cp "$bin_path" dist/spider-enterprise-crawler
  # 添加执行权限
  chmod +x dist/spider-enterprise-crawler
  log_info "二进制构建完成，路径：dist/spider-enterprise-crawler"
}

# Bazel打包全量Bundle（含配置、脚本、依赖）
build_bundle() {
  log_info "开始构建企业级分发包..."
  bazel build "$BAZEL_BUNDLE_TARGET"
  
  # 解压Bundle到dist目录（模拟生产级打包）
  local bundle_path=$(bazel cquery "$BAZEL_BUNDLE_TARGET" --output=files)
  tar -zxf "$bundle_path" -C dist/
  # 补充项目必要文件
  cp config/crawler.yaml dist/config/ || log_warn "配置文件 config/crawler.yaml 复制失败"
  cp .env.example dist/ || log_warn "环境变量模板 .env.example 复制失败"
  cp run_crawler.sh dist/ || log_warn "启动脚本 run_crawler.sh 复制失败"
  cp deploy/docker-compose.yml dist/deploy/ || log_warn "Docker Compose配置复制失败"
  chmod +x dist/run_crawler.sh
  log_info "分发包构建完成，路径：dist/ (包含二进制+配置+启动脚本+部署文件)"
}

# 构建Docker镜像（适配项目Docker配置）
build_docker() {
  log_info "开始构建Docker镜像: ${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}..."
  # 方式1：使用Bazel容器规则（推荐，标准化构建）
  bazel build //:crawler-image \
    --define "IMAGE_REGISTRY=${DOCKER_REGISTRY}" \
    --define "IMAGE_TAG=${IMAGE_TAG}"
  
  # 方式2：原生Docker构建（兼容备选，适配项目Dockerfile）
  # docker build -t "${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}" .
  
  log_info "Docker镜像构建完成：${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"
}

# 推送Docker镜像
push_docker() {
  log_info "推送Docker镜像到仓库..."
  docker push "${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"
  log_info "镜像推送完成：${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"
}

# K8s部署（适配项目K8s配置）
deploy_k8s() {
  log_info "开始K8s部署（命名空间：${K8S_NAMESPACE}）..."
  
  # 创建命名空间（不存在则创建）
  kubectl create namespace "${K8S_NAMESPACE}" --dry-run=client -o yaml | kubectl apply -f -
  
  # 创建ConfigMap（从deploy/k8s/configmap.yaml）
  kubectl apply -f deploy/k8s/configmap.yaml -n "${K8S_NAMESPACE}"
  
  # 加载.env文件中的敏感配置，创建Secret
  if [[ -f .env ]]; then
    source .env
    kubectl create secret generic "${K8S_SECRET}" \
      --namespace "${K8S_NAMESPACE}" \
      --from-literal=LOGIN_USERNAME="${LOGIN_USERNAME:-}" \
      --from-literal=LOGIN_PASSWORD="${LOGIN_PASSWORD:-}" \
      --from-literal=MINIO_ACCESS_KEY="${MINIO_ACCESS_KEY:-}" \
      --from-literal=MINIO_SECRET_KEY="${MINIO_SECRET_KEY:-}" \
      --dry-run=client -o yaml | kubectl apply -f -
  else
    log_error "未找到.env文件，无法创建K8s Secret"
  fi
  
  # 替换镜像Tag并部署CronJob
  sed -i.bak "s|your-registry/${IMAGE_NAME}:.*|${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}|g" deploy/k8s/cronjob.yaml
  kubectl apply -f deploy/k8s/cronjob.yaml -n "${K8S_NAMESPACE}"
  rm -f deploy/k8s/cronjob.yaml.bak  # 清理临时文件
  
  # 验证部署状态
  log_info "验证K8s资源状态..."
  kubectl get cronjob "${K8S_CRONJOB}" -n "${K8S_NAMESPACE}" || log_warn "CronJob部署验证失败"
  kubectl get configmap "${K8S_CONFIGMAP}" -n "${K8S_NAMESPACE}" || log_warn "ConfigMap验证失败"
  kubectl get secret "${K8S_SECRET}" -n "${K8S_NAMESPACE}" || log_warn "Secret验证失败"
  
  log_info "K8s部署完成！每日凌晨2点将自动执行爬虫任务"
}

# 启动基础依赖服务（Redis/MinIO，适配docker-compose）
start_deps() {
  log_info "启动Redis/MinIO依赖服务..."
  sudo docker compose -f deploy/docker-compose.yml up -d redis minio
  # 等待服务就绪
  sleep 10
  # 验证服务状态
  if ! docker ps | grep -q "spider-rs-minio"; then
    log_warn "MinIO服务启动失败，请检查docker-compose配置"
  fi
  if ! docker ps | grep -q "redis"; then
    log_warn "Redis服务启动失败，请检查docker-compose配置"
  fi
  log_info "基础依赖服务启动完成"
}

# 显示帮助信息
show_help() {
  cat << EOF
使用说明：$0 [选项]
基于Bazel的爬虫项目全生命周期构建/打包/部署脚本（适配spider-rs-crawler）

核心选项：
  --release          以Release模式构建（默认Debug）
  --bundle           构建全量分发包（含二进制+配置+脚本+部署文件）
  --docker           构建Docker镜像
  --push             推送Docker镜像（需先指定--docker）
  --deploy           部署到K8s集群（需先指定--docker/--push）
  --start-deps       启动Redis/MinIO基础依赖（docker-compose）
  --clean            仅清理旧构建产物
  --help             显示帮助信息

使用示例：
  1. 仅构建Debug二进制：$0
  2. 构建Release+分发包+启动依赖：$0 --release --bundle --start-deps
  3. 构建镜像+推送+K8s部署：$0 --release --docker --push --deploy
  4. 全流程（构建→打包→镜像→推送→部署）：$0 --release --bundle --docker --push --deploy
  5. 仅清理产物：$0 --clean
EOF
}

# ===================== 主流程 =====================
# 初始化功能开关
ENABLE_DOCKER="false"
ENABLE_PUSH="false"
ENABLE_DEPLOY="false"
ENABLE_BUNDLE="false"
ENABLE_START_DEPS="false"

# 解析命令行参数
while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)
      BUILD_MODE="release"
      shift
      ;;
    --bundle)
      ENABLE_BUNDLE="true"
      shift
      ;;
    --docker)
      ENABLE_DOCKER="true"
      shift
      ;;
    --push)
      ENABLE_PUSH="true"
      shift
      ;;
    --deploy)
      ENABLE_DEPLOY="true"
      shift
      ;;
    --start-deps)
      ENABLE_START_DEPS="true"
      shift
      ;;
    --clean)
      clean_old_build
      exit 0
      ;;
    --help)
      show_help
      exit 0
      ;;
    *)
      log_error "未知参数：$1，请执行 $0 --help 查看用法"
      ;;
  esac
done

# 主执行流程
log_info "===== 开始执行spider-rs-crawler全生命周期构建 ====="

# 1. 环境校验
check_dependencies

# 2. 清理旧产物
clean_old_build

# 3. 启动基础依赖（可选）
if [[ "$ENABLE_START_DEPS" == "true" ]]; then
  start_deps
fi

# 4. 构建二进制
build_binary

# 5. 构建分发包（可选）
if [[ "$ENABLE_BUNDLE" == "true" ]]; then
  build_bundle
fi

# 6. 构建Docker镜像（可选）
if [[ "$ENABLE_DOCKER" == "true" ]]; then
  build_docker
  
  # 推送镜像（可选）
  if [[ "$ENABLE_PUSH" == "true" ]]; then
    push_docker
  fi
fi

# 7. K8s部署（可选）
if [[ "$ENABLE_DEPLOY" == "true" ]]; then
  deploy_k8s
fi

# 执行完成汇总
log_info "===== 全生命周期流程执行完成 ====="
log_info "关键产物路径："
log_info "  - 二进制文件：dist/spider-enterprise-crawler"
if [[ "$ENABLE_BUNDLE" == "true" ]]; then
  log_info "  - 全量分发包：dist/"
fi
if [[ "$ENABLE_DOCKER" == "true" ]]; then
  log_info "  - Docker镜像：${DOCKER_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"
fi
if [[ "$ENABLE_DEPLOY" == "true" ]]; then
  log_info "  - K8s部署命名空间：${K8S_NAMESPACE}"
fi
if [[ "$ENABLE_START_DEPS" == "true" ]]; then
  log_info "  - 基础依赖：Redis/MinIO（已通过docker-compose启动）"
fi
log_info "使用提示：可执行 ./dist/run_crawler.sh 启动爬虫，或通过K8s CronJob自动执行"