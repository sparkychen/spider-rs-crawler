use anyhow::{Context, Result};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct CrawlerConfig {
    pub redis: RedisConfig,
    pub login: LoginConfig,
    pub cookie: CookieConfig,
    pub crawl: CrawlConfig,
    pub request_delay: u64,
    pub proxy: String,
    pub user_agent: String,
    pub chrome: ChromeConfig,
    pub storage: StorageConfig,
    pub monitor: MonitorConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoginConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    pub username_selector: String,
    pub password_selector: String,
    pub login_btn_selector: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CookieConfig {
    pub key: String,
    pub ttl: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CrawlConfig {
    pub target_url: String,
    pub depth: u32,
    pub concurrency: usize,
    pub use_chrome: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChromeConfig {
    pub headless: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StorageConfig {
    pub minio: MinioConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MinioConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MonitorConfig {
    pub metrics_port: u16,
    pub log_level: String,
}

/// 加载配置文件，替换环境变量
pub fn load_config(path: &str) -> Result<CrawlerConfig> {
    let mut config = config::Config::builder()
        .add_source(config::File::with_name(path))
        .build()?
        .try_deserialize::<CrawlerConfig>()?;

    // 替换环境变量（${VAR}格式）
    config.login.username = replace_env_var(&config.login.username)?;
    config.login.password = replace_env_var(&config.login.password)?;
    config.storage.minio.access_key = replace_env_var(&config.storage.minio.access_key)?;
    config.storage.minio.secret_key = replace_env_var(&config.storage.minio.secret_key)?;

    // 设置日志级别环境变量
    env::set_var("RUST_LOG", &config.monitor.log_level);

    Ok(config)
}

/// 替换${VAR}格式的环境变量
fn replace_env_var(s: &str) -> Result<String> {
    let mut result = s.to_string();
    let re = regex::Regex::new(r"\$\{([A-Za-z0-9_]+)\}").unwrap();
    for cap in re.captures_iter(s) {
        let var_name = cap.get(1).unwrap().as_str();
        let var_value = env::var(var_name)
            .with_context(|| format!("Environment variable {} not found", var_name))?;
        result = result.replace(&cap[0], &var_value);
    }
    Ok(result)
}
