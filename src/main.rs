// 1. 基础依赖导入
use anyhow::{Context, Result};
use redis::AsyncCommands;
use serde::Deserialize;
use tracing::info;
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

// ========== 配置结构体 ==========
#[derive(Deserialize, Debug)]
struct Config {
    crawl: CrawlConfig,
    redis: RedisConfig,
    cookie: String,
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

// ========== 核心爬取逻辑（修复最后一个类型错误） ==========
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

    // 3. 缓存Cookie
    let cookie_key = "crawler:cookie";
    let cached_cookie: Option<String> = redis_conn.get(cookie_key).await?;
    let current_cookie = match cached_cookie {
        Some(c) => {
            info!("✅ 从Redis加载Cookie");
            c
        }
        None => {
            redis_conn.set_ex::<_, _, ()>(cookie_key, &config.cookie, config.redis.cookie_ttl).await?;
            info!("✅ Cookie存入Redis（有效期：{}秒）", config.redis.cookie_ttl);
            config.cookie
        }
    };

    // 4. 开始爬取
    start_crawling(&config.crawl, &current_cookie).await?;

    Ok(())
}

