#一、方案概述
本方案基于 Rust 生态的spider-rs爬虫库（原生支持 Chrome DevTools Protocol / 真实浏览器模拟），结合 Bazel 构建工具实现标准化打包 分发，解决需要登录状态的网站爬取需求，同时满足企业级的可靠性、可维护性、可观测性要求。
核心优势： 

    基于真实浏览器（Chrome Headless）模拟登录，绕过前端反爬 / 登录验证；
    Bazel 实现跨平台、增量构建，适配企业级 CI/CD 流程；
    支持登录态持久化、任务调度、监控告警、反爬对抗等企业级特性；
    复用 spider-rs 原生的并发、拦截、指纹模拟能力。

# 安装依赖（注：程序是基于已在使用unbutu-24.04.05，在配置该程序之前已经安装过一些依赖包或库程序，如果在其他机器上部署该程序有错误请具体问题具体分析后解决相关Issuses）  
##安装stable Rust
sudo apt install -y gcc-11 g++-11
export CC=gcc-11 CXX=g++-11
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install -y xvfb  

## 验证安装  
rustc --version  
cargo --version  

## 在根目录.env配置要登陆的html账户密码（项目配置文件中映射该账户配置， 测试基于本人邮箱）：  
export LOGIN_USERNAME=cjf_junfeng@163.com  
export LOGIN_PASSWORD=****************  

## 安装 npm node
curl -o- https://gh-proxy.org/https://github.com/nvm-sh/nvm/blob/v0.40.4/install.sh | bash  
# wget -qO- https://gh-proxy.org/https://github.com/nvm-sh/nvm/blob/v0.40.4/install.sh | bash    
然后将下面内容配置到~/.bashrc    
export NVM_DIR="$HOME/.nvm"    
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"  # This loads nvm    
[ -s "$NVM_DIR/bash_completion" ] && \. "$NVM_DIR/bash_completion"  # This loads nvm bash_completion    
 再执行如下命令  
source ~/.bashrc  

## 安装 node  
nvm install 12.22.12  
nvm use 12.22.12  

## 安装bazel  
wget "https://gh-proxy.org/https://github.com/bazelbuild/bazel/releases/download/7.0.0/bazel-7.0.0-linux-x86_64"  
cp bazel-7.0.0-linux-x86_64 .nvm/versions/node/v12.22.12/bin/bazel  
之后验证bzael是否配置成功（若显示版本则成功）：  
bazel --version    

## 在项目配置文件config/crawler.yaml中配置登陆url和爬取html url:  
login:    
  url: "https://mail.163.com/"    
crawl:    
  target_url: "https://mail.163.com/"    

## 单机配置爬取url文件(下载)目录：  
crawl:    
  target_url: "https://mail.163.com/"      
  download_dir: "downloads/163mail"    # 自定义下载目录（可选，默认：downloads/）    

# 二、启动  
## 1、命令行启动，启动redis等 docker镜像  
sudo docker compose -f deploy/docker-compose.yml up -d redis minio  

## 2、手动执行爬虫程序  
cargo run -- --config config/crawler.yaml  

## 3、手动执行爬虫程序，但不在爬取网页时自动打开爬取网页  
xvfb-run --auto-servernum target/debug/spider-enterprise-crawler --config config/crawler.yaml  

## 3、从脚本执行  
./run_crawler.sh  

# scripts/build.sh使用说明
参数： 
--help	显示帮助信息（参数说明 + 使用示例）	无依赖  
--release	以 Release 模式构建（优化编译，产物体积更小、性能更高）	默认是 Debug 模式  
--bundle	构建全量分发包（含二进制、配置文件、启动脚本、部署文件，输出到 dist/）	依赖 Bazel 构建完成  
--docker	基于 Bazel 构建 Docker 镜像（镜像名 / Tag 对应脚本内 DOCKER_REGISTRY 等配置需提前安装 Docker，且 Docker 服务已启动   
--push	推送构建好的 Docker 镜像到指定镜像仓库	必须先指定 --docker，且已登录镜像仓库  
--deploy	将爬虫部署到K8s集群（创建命名空间、ConfigMap、Secret、CronJob必须先指定 --docker/--push，且 Kubectl已配置集群权限    
--start-deps	一键启动 Redis/MinIO 基础依赖（基于 deploy/docker-compose.yml）	需安装 Docker Compose，且有 sudo 权限  
--clean	仅清理旧构建产物（Bazel 缓存、Rust 产物、日志 / 临时文件等）	无依赖，执行后直接退出 

示例：  
### 清理 Bazel 缓存、target/、日志、临时文件等
scripts/build.sh --clean

### 校验环境 → 清理旧产物 → 构建 Debug 二进制（输出到 dist/spider-enterprise-crawler）
scripts/build.sh

### 以 Release 模式构建，同时生成全量分发包（dist/ 下包含二进制+配置+部署文件）
scripts/build.sh --release --bundle

## 执行二进制，验证配置加载正常
./dist/spider-enterprise-crawler --config config/crawler.yaml

######################################################################################## 我ubuntu-24.04本机上没有装没有kubectl相关组件，暂时不支持编译docker镜像  
### # 构建 Release 二进制 → 构建 Docker 镜像（仅保存在本地，不推送）** --docker暂不支持
scripts/build.sh --release --docker

### 先登录镜像仓库（示例：Docker Hub）
docker login docker.io -u 你的用户名 -p 你的密码
### 构建 Release 镜像 → 推送至指定仓库
scripts/build.sh --release --docker --push

## 全流程（构建 → 打包 → 镜像 → 推送 → K8s 部署）
# 1. 前置：确保 Kubectl 已配置目标 K8s 集群（kubectl config use-context 你的集群）
# 2. 登录镜像仓库
docker login 你的镜像仓库地址 -u 用户名 -p 密码
# 3. 全流程执行：启动依赖 → 构建 Release → 打包 → 构建镜像 → 推送 → 部署到 K8s
scripts/build.sh --release --bundle --start-deps --docker --push --deploy
