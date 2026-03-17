use anyhow::{Context, Result};
use spider::chrome::{ChromeConfig, ChromeEvent, ChromeEventTracker};
use spider::configuration::{Configuration, Fingerprint, WaitForIdleNetwork, WaitForSelector};
use spider::website::WebsiteBuilder;
use std::time::Duration;
use crate::config::CrawlerConfig;
use crate::login::cookie_store::CookieStore;

/// Chrome模拟登录并持久化Cookie
pub async fn chrome_login(config: &CrawlerConfig) -> Result<()> {
    // 构建Chrome配置（适配spider-rs v2.45）
    let chrome_config = ChromeConfig::builder()
        .headless(config.chrome.headless)
        .headless_new(true)
        .timeout(Duration::from_secs(60))
        .proxy(config.proxy.clone().filter(|s| !s.is_empty()))
        .build()
        .context("Failed to build Chrome config")?;

    // 构建爬虫基础配置
    let mut crawler_config = Configuration::new();
    crawler_config
        .set_depth(1)
        .set_delay(config.request_delay)
        .set_stealth(true)
        .set_fingerprint(Fingerprint::Random)
        .set_wait_for_idle_network(Some(WaitForIdleNetwork::new(Some(Duration::from_secs(30)))))
        .set_wait_for_selector(Some(WaitForSelector::new(Some(Duration::from_secs(10)), "body".into())));

    // 构建Website实例（v2.45 新API）
    let mut website = WebsiteBuilder::new(&config.login.url)
        .with_config(crawler_config)
        .with_chrome_config(Some(chrome_config))
        .with_event_tracker(Some(ChromeEventTracker::new(true, true)))
        .build()
        .context("Failed to build Website")?;

    // 注册登录自动化脚本
    website.register_automation_script(
        &config.login.url,
        vec![
            ChromeEvent::Wait(Duration::from_secs(2)),
            ChromeEvent::Type(config.login.username_selector.clone(), config.login.username.clone()),
            ChromeEvent::Wait(Duration::from_secs(1)),
            ChromeEvent::Type(config.login.password_selector.clone(), config.login.password.clone()),
            ChromeEvent::Wait(Duration::from_secs(1)),
            ChromeEvent::Click(config.login.login_btn_selector.clone()),
            ChromeEvent::WaitForIdleNetwork(Duration::from_secs(30)),
        ],
    );

    // 执行登录流程
    tracing::info!("Starting login to: {}", config.login.url);
    website.crawl().await.context("Login crawl failed")?;

    // 持久化Cookie到Redis
    let cookies = website.get_cookies().await.context("Failed to get cookies")?;
    let cookie_store = CookieStore::new(&config.redis.url).await?;
    cookie_store
        .set_cookies(&config.cookie.key, &cookies, Duration::from_secs(config.cookie.ttl))
        .await
        .context("Failed to persist cookies")?;

    tracing::info!("Login success, cookies persisted to Redis (TTL: {}s)", config.cookie.ttl);
    Ok(())
}
