// 1. 基础依赖导入
use anyhow::{Context, Result};
use redis::AsyncCommands;
use serde::Deserialize;
use tracing::{info, warn, error};
use std::process;
use std::path::Path;
use urlencoding::encode;
// 2. 正则+懒加载依赖
use regex::Regex;
use lazy_static::lazy_static;
// 3. Spider核心类型
use spider::client::header::{HeaderMap, HeaderValue, COOKIE};
use spider::client::StatusCode;
use spider::configuration::Configuration;
use spider::page::Page;
use spider::website::Website;
use spider::cookie::Cookie;
use std::time::Duration;
// ========== 新增：引入login模块（关键） ==========
mod login;
use login::chrome_login::chrome_login;
use login::cookie_store::CookieStore;

// ========== 配置结构体 ==========
#[derive(Deserialize, Debug)]
struct Config {
    crawl: CrawlConfig,
    redis: RedisConfig,
    cookie: String,
    login: LoginConfig,      // 新增：登录配置
    chrome: ChromeConfig,    // 新增：Chrome配置
    cookie_key: String,      // 新增：Cookie存储的Redis Key
    proxy: String,           // 新增：代理配置
    request_delay: u64,      // 新增：请求延迟
}

#[derive(Deserialize, Debug)]
struct CrawlConfig {
    target_url: String,
    depth: usize,
    concurrency: usize,
    download_dir: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RedisConfig {
    url: String,
    cookie_ttl: u64,
}

// ========== 新增：登录/Chrome配置结构体（匹配yaml） ==========
#[derive(Deserialize, Debug, Clone)]
struct LoginConfig {
    url: String,
    username: String,
    password: String,
    username_selector: String,
    password_selector: String,
    login_btn_selector: String,
}

#[derive(Deserialize, Debug, Clone)]
struct ChromeConfig {
    headless: bool,
}

// ========== 工具函数：创建目录 ==========
async fn create_dir_if_not_exists(dir: &str) -> Result<()> {
    if !Path::new(dir).exists() {
        tokio::fs::create_dir_all(dir)
            .await
            .context(format!("❌ 创建目录 {} 失败", dir))?;
        info!("✅ 目录 {} 创建成功", dir);
    }
    Ok(())
}

// ========== 核心：正则解析标题 ==========
lazy_static! {
    static ref TITLE_REGEX: Regex = Regex::new(r"(?i)<title>([\s\S]*?)</title>").unwrap();
}

// 函数参数兼容String/&str：接收&str
fn get_title_from_html(html: &str) -> String {
    match TITLE_REGEX.captures(html) {
        Some(caps) => {
            let title = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("无标题");
            title.to_string()
        }
        None => "无标题".to_string(),
    }
}

// ========== 保存HTML页面 ==========
async fn save_page(page: &Page, download_dir: &str) -> Result<()> {
    let url = page.get_url();
    let encoded_url = encode(url);
    let filename = format!("{}.html", &encoded_url.chars().take(50).collect::<String>());
    let file_path = Path::new(download_dir).join(filename);

    // 适配page.get_html()返回String的情况
    let html = page.get_html();
    tokio::fs::write(&file_path, html)
        .await
        .context(format!("❌ 保存页面 {} 失败", url))?;
    
    info!("📥 页面已下载：{} -> {}", url, file_path.display());
    Ok(())
}

// ========== 加载配置 ==========
async fn load_config_file(path: &str) -> Result<Config> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context(format!("❌ 读取配置 {} 失败", path))?;
    
    serde_yaml::from_str(&content).context("❌ 解析YAML失败")
}

// ========== 新增：Cookie转字符串工具函数 ==========
fn cookies_to_string(cookies: &[Cookie]) -> String {
    cookies
        .iter()
        .map(|c| format!("{}={}", c.name(), c.value()))
        .collect::<Vec<_>>()
        .join("; ")
}

// ========== 核心爬取逻辑 ==========
async fn start_crawling(cfg: &CrawlConfig, cookie: &str) -> Result<()> {
    info!("🕷️ 开始爬取：{}", cfg.target_url);

    // 爬虫配置
    let mut config = Configuration::new();
    config
        .with_depth(cfg.depth)
        .with_concurrency_limit(Some(cfg.concurrency))
        .with_user_agent(Some("Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:123.0) Gecko/20100101 Firefox/123.0"))
        .with_delay(1000);

    // 初始化爬取器
    let mut website = Website::new(&cfg.target_url);
    website.with_config(config);

    // 添加Cookie
    let mut headers = HeaderMap::new();
    if let Ok(cookie_val) = HeaderValue::from_str(cookie) {
        headers.insert(COOKIE, cookie_val);
    }
    website.with_headers(Some(headers));

    // 订阅结果 + 初始化下载目录
    let rx = website.subscribe(cfg.concurrency).expect("❌ 订阅失败");
    let download_dir = cfg.download_dir.as_deref().unwrap_or("downloads");
    create_dir_if_not_exists(download_dir).await?;

    // 开始爬取
    website.crawl().await;

    // 处理爬取结果（核心修复：强制将html转为&str）
    let mut count = 0;
    let mut rx_mut = rx;
    while let Ok(page) = rx_mut.recv().await {
        count += 1;
        let url = page.get_url().to_string();
        let html = page.get_html(); // 你的版本返回String
        
        // ✅ 最后修复：强制传递&str（不管html是String还是&str）
        let title = get_title_from_html(&html); 

        // 打印信息
        info!("\n===== 页面 {} =====", count);
        info!("URL: {}", url);
        info!("标题: {}", title);
        info!("HTML长度: {} 字节", html.len());
        info!("前200字符: {}", &html.chars().take(200).collect::<String>());
        
        // 状态码
        let status_code = if page.status_code == StatusCode::default() {
            0
        } else {
            page.status_code.as_u16()
        };
        info!("状态码: {}", status_code);
        info!("=======================\n");

        // 保存页面
        save_page(&page, download_dir).await?;
    }

    info!("🎉 爬取完成 | 共 {} 页 | 保存至: {}", count, download_dir);
    Ok(())
}

// ========== 主函数 ==========
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("spider_enterprise_crawler=info")
        .init();
    // 1. 加载配置
    let config = load_config_file("config/crawler.yaml").await?;
    info!("✅ 配置加载成功");

    // 2. 连接Redis
    let redis_client = redis::Client::open(config.redis.url.clone()).context("❌ Redis连接失败")?;
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
    info!("✅ Redis连接成功");

    // 3. 处理Cookie（核心修改：新增模拟登录逻辑）
    let cookie_store = CookieStore::new(&config.redis.url).await?;
    let current_cookie: String = match cookie_store.is_cookie_valid(&config.cookie_key).await {
        // 情况1：Redis中有有效Cookie，直接加载
        Ok(true) => {
            let cookies = cookie_store.get_cookies(&config.cookie_key).await.context("❌ 读取Redis Cookie失败")?;
            let cookie_str = cookies_to_string(&cookies);
            info!("✅ 从Redis加载有效Cookie");
            cookie_str
        }
        // 情况2：Cookie无效/不存在/Redis异常，执行Chrome模拟登录
        _ => {
            warn!("⚠️ Redis中无有效Cookie，执行Chrome登录...");
            // 调用chrome_login.rs的核心登录函数（增强错误处理）
            let cookies = match chrome_login(&config).await {
                Ok(c) => c,
                Err(e) => {
                    // 登录失败：打印详细错误信息 + 终止程序
                    error!("==================================================");
                    error!("❌ Chrome登录失败，爬取流程终止！");
                    error!("📌 登录目标URL：{}", config.login.url);
                    error!("📌 登录账号：{}", config.login.username);
                    error!("📌 失败原因：{}", e);
                    error!("==================================================");
                    // 显式退出程序，状态码1表示执行失败
                    process::exit(1);
                }
            };
            let cookie_str = cookies_to_string(&cookies);
            // 登录成功后，将Cookie存入Redis
            redis_conn.set_ex::<_, _, ()>(
                &config.cookie_key,
                &cookie_str,
                config.redis.cookie_ttl
            ).await.context("❌ Cookie存入Redis失败")?;
            info!("✅ 模拟登录成功，Cookie已缓存（有效期：{}秒）", config.redis.cookie_ttl);
            cookie_str
        }
    };

    // 4. 开始爬取
    start_crawling(&config.crawl, &current_cookie).await?;

    Ok(())
}

