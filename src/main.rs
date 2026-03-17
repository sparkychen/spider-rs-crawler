// 1. 基础依赖导入
use anyhow::{Context, Result};
use redis::AsyncCommands;
use serde::Deserialize;
use tracing::{info, warn, error};
use std::process;
use std::path::Path;
use urlencoding::encode;
// 新增：解析.env和替换环境变量
use dotenv::dotenv;
use std::env;
use regex::Regex;
// 2. 正则+懒加载依赖
use lazy_static::lazy_static;
// 3. Spider核心类型
use spider::client::header::{HeaderMap, HeaderValue, COOKIE};
// use spider::client::StatusCode;
use spider::configuration::Configuration;
use spider::page::Page;
use spider::website::Website;
// 4. Cookie+模块导入
use cookie::Cookie as ExternalCookie;
mod login;
use login::{CookieStore, login_with_chrome};

// ========== 环境变量替换工具函数 ==========
fn replace_env_vars(s: &str) -> String {
    lazy_static! {
        static ref ENV_REGEX: Regex = Regex::new(r"\$\{([A-Za-z0-9_]+)\}").unwrap();
    }
    ENV_REGEX.replace_all(s, |caps: &regex::Captures| {
        let var_name = &caps[1];
        env::var(var_name).unwrap_or_else(|_| format!("${{{}}}", var_name))
    }).to_string()
}

// ========== 统一Config结构体 ==========
#[derive(Deserialize, Debug, Clone)]
pub struct CrawlConfig {
    pub target_url: String,
    pub depth: usize,
    pub concurrency: usize,
    pub use_chrome: Option<bool>,
    pub download_dir: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub cookie_ttl: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LoginConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    pub username_selector: String,
    pub password_selector: String,
    pub login_btn_selector: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChromeConfig {
    pub headless: bool,
    pub executable_path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MinioConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct StorageConfig {
    pub minio: MinioConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MonitorConfig {
    pub metrics_port: u16,
    pub log_level: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub redis: RedisConfig,
    pub login: LoginConfig,
    pub cookie: String,
    pub cookie_key: String,
    pub chrome_headless: Option<bool>,
    pub cookie_ttl_seconds: Option<u64>,
    pub chrome: ChromeConfig,
    pub crawl: CrawlConfig,
    pub request_delay: u64,
    pub proxy: String,
    pub user_agent: Option<String>,
    pub storage: Option<StorageConfig>,
    pub monitor: Option<MonitorConfig>,
}

// ========== 加载配置（含环境变量替换） ==========
async fn load_config_file(path: &str) -> Result<Config> {
    // 加载.env文件
    dotenv().ok();
    
    // 读取YAML文件
    let content = tokio::fs::read_to_string(path)
        .await
        .context(format!("❌ 读取配置文件 {} 失败", path))?;
    
    // 替换环境变量占位符
    let content_with_env = replace_env_vars(&content);
    
    // 解析YAML
    let mut config: Config = serde_yaml::from_str(&content_with_env)
        .context("❌ 解析YAML配置失败")?;
    
    // 二次替换（防止嵌套环境变量）
    config.login.username = replace_env_vars(&config.login.username);
    config.login.password = replace_env_vars(&config.login.password);
    config.storage = config.storage.map(|mut s| {
        s.minio.access_key = replace_env_vars(&s.minio.access_key);
        s.minio.secret_key = replace_env_vars(&s.minio.secret_key);
        s
    });

    Ok(config)
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

    let html = page.get_html();
    tokio::fs::write(&file_path, html)
        .await
        .context(format!("❌ 保存页面 {} 失败", url))?;
    
    info!("📥 页面已下载：{} -> {}", url, file_path.display());
    Ok(())
}

// ========== 核心爬取逻辑 ==========
// 修复后：传入顶级Config的request_delay/user_agent
async fn start_crawling(
    crawl_cfg: &CrawlConfig, 
    cookie: &str, 
    request_delay: u64,  // ✅ 新增：顶级request_delay
    user_agent: Option<&str>  // ✅ 新增：顶级user_agent
) -> Result<()> {
    info!("🕷️ 开始爬取目标URL：{}", crawl_cfg.target_url);

    let mut config = Configuration::new();
    config
        .with_depth(crawl_cfg.depth)
        .with_concurrency_limit(Some(crawl_cfg.concurrency))
        .with_delay(request_delay); // ✅ 修正：使用传入的request_delay（不再用cfg.request_delay）

    // 设置User-Agent（修复类型不匹配+字段归属）
    if let Some(ua) = user_agent { // ✅ 修正：使用传入的user_agent
        config.with_user_agent(Some(ua)); // ✅ 符合Option<&str>类型
    } else {
        // ✅ 修复：用Some包裹字符串，匹配Option<&str>类型
        config.with_user_agent(Some("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));
    }

    let mut website = Website::new(&crawl_cfg.target_url);
    website.with_config(config);

    // 添加Cookie（无变化）
    let mut headers = HeaderMap::new();
    if let Ok(cookie_val) = HeaderValue::from_str(cookie) {
        headers.insert(COOKIE, cookie_val);
    }
    website.with_headers(Some(headers));

    let rx = website.subscribe(crawl_cfg.concurrency).expect("❌ 订阅爬虫结果失败");
    let download_dir = crawl_cfg.download_dir.as_deref().unwrap_or("downloads");
    create_dir_if_not_exists(download_dir).await?;

    // 开始爬取（无变化）
    website.crawl().await;

    // 处理爬取结果（仅把cfg改为crawl_cfg，其余不变）
    let mut count = 0;
    let mut rx_mut = rx;
    while let Ok(page) = rx_mut.recv().await {
        count += 1;
        let url = page.get_url().to_string();
        let html = page.get_html();
        
        let title = get_title_from_html(&html); 

        // 打印信息（cfg → crawl_cfg）
        info!("\n===== 爬取结果 #{} =====", count);
        info!("URL: {}", url);
        info!("标题: {}", title);
        info!("页面大小: {} 字节", html.len());
        info!("状态码: {}", page.status_code.as_u16()); // ✅ 若用不到StatusCode，可删掉这行
        info!("=======================\n");

        // 保存页面（cfg → crawl_cfg）
        save_page(&page, download_dir).await?;
    }

    info!("🎉 爬取完成 | 共爬取 {} 个页面 | 保存至: {}", count, download_dir);
    Ok(())
}

// ========== 主函数（强化登录失败处理） ==========
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter("spider_enterprise_crawler=info")
        .init();
    
    // 1. 加载配置（含.env解析）
    let config: Config = load_config_file("config/crawler.yaml").await?;
    info!("✅ 配置加载成功（已解析.env环境变量）");
    info!("🔍 待验证的登录账号：{}", config.login.username);

    // 2. 连接Redis
    let redis_client = redis::Client::open(config.redis.url.clone())
        .context("❌ Redis连接失败")?;
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
    info!("✅ Redis连接成功（地址：{}）", config.redis.url);

    // 3. 处理Cookie（核心：错误账号密码强制失败）
    let cookie_store: CookieStore = CookieStore::new(&config.redis.url).await?;
    let current_cookie: String = match cookie_store.is_cookie_valid(&config.cookie_key).await {
        // 情况1：Redis中有有效Cookie，直接加载
        Ok(true) => {
            let cookies: Vec<ExternalCookie<'static>> = cookie_store.get_cookies(&config.cookie_key).await
                .context("❌ 读取Redis Cookie失败")?;
            let cookie_str = cookies.iter()
                .map(|c| format!("{}={}", c.name(), c.value()))
                .collect::<Vec<_>>()
                .join("; ");
            info!("✅ 从Redis加载有效Cookie（有效期剩余：{}秒）", config.redis.cookie_ttl);
            cookie_str
        }
        // 情况2：执行登录（错误账号密码强制失败）
        _ => {
            warn!("⚠️ Redis中无有效Cookie，执行登录验证...");
            // 调用登录函数 + 强制失败处理
            let cookies: Vec<ExternalCookie<'static>> = match login_with_chrome(&config).await {
                Ok(c) => c,
                Err(e) => {
                    // 强化登录失败处理：打印详细错误 + 退出程序
                    error!("\n======================= 登录失败 =======================");
                    error!("🚨 账号密码验证失败，程序终止！");
                    error!("📝 失败原因：{}", e);
                    error!("🔍 验证账号：{}", config.login.username);
                    error!("=======================================================\n");
                    process::exit(1); // 非0状态码表示失败
                }
            };
            // 登录成功：存入Redis
            let cookie_str = cookies.iter()
                .map(|c| format!("{}={}", c.name(), c.value()))
                .collect::<Vec<_>>()
                .join("; ");
            redis_conn.set_ex::<_, _, ()>(
                &config.cookie_key,
                &cookie_str,
                config.redis.cookie_ttl
            ).await.context("❌ Cookie存入Redis失败")?;
            info!("✅ 登录成功，Cookie已缓存至Redis（有效期：{}秒）", config.redis.cookie_ttl);
            cookie_str
        }
    };

    // 修复后的调用
    start_crawling(
        &config.crawl, 
        &current_cookie, 
        config.request_delay,
        config.user_agent.as_deref()
    ).await?;

    Ok(())
}
