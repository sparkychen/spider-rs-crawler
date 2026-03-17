#一、方案概述
本方案基于 Rust 生态的spider-rs爬虫库（原生支持 Chrome DevTools Protocol / 真实浏览器模拟），结合 Bazel 构建工具实现标准化打包分发，解决需要登录状态的网站爬取需求，同时满足企业级的可靠性、可维护性、可观测性要求。
核心优势：

    基于真实浏览器（Chrome Headless）模拟登录，绕过前端反爬 / 登录验证；
    Bazel 实现跨平台、增量构建，适配企业级 CI/CD 流程；
    支持登录态持久化、任务调度、监控告警、反爬对抗等企业级特性；
    复用 spider-rs 原生的并发、拦截、指纹模拟能力。

# 安装依赖（注：程序是基于已在使用unbutu-24.04.05，在配置该程序之前已经安装过一些依赖包或库程序，如果在其他机器上部署该程序有错误请具体问题具体分析后解决相关Issuses）
##安装stable Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install -y xvfb

## 验证安装
rustc --version
cargo --version

## 在根目录.env配置要登陆的html账户密码（项目配置文件中映射该账户配置）：
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

## 单机配置爬取url文件目录：
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

