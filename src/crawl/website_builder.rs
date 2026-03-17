use anyhow::{Context, Result};
use spider::configuration::{Configuration, Fingerprint, WaitForIdleNetwork, WaitForSelector};
use spider::cookie::Cookie;
use spider::website::WebsiteBuilder;
use std::time::Duration;
use crate::config::CrawlerConfig;
use crate::login::cookie_store::CookieStore;

/// 构建带登录态的爬虫实例
pub async fn build_authed_website(config: &CrawlerConfig) -> Result<spider::website::Website> {
    // 加载Cookie
    let cookie_store = CookieStore::new(&config.redis.url).await?;
    let cookies = match cookie_store.get_cookies(&config.cookie.key).await {
        Ok(c) if !c.is_empty() => c,
        _ => {
            tracing::info!("Cookies invalid/expired, trigger re-login");
            crate::login::chrome_login::chrome_login(config).await?;
            cookie_store.get_cookies(&config.cookie.key).await?
        }
    };

    // 构建爬虫配置
    let mut crawler_config = Configuration::new();
    crawler_config
        .set_depth(config.crawl.depth)
        .set_delay(config.request_delay)
        .set_concurrency(config.crawl.concurrency)
        .set_stealth(true)
        .set_fingerprint(Fingerprint::Random)
        .set_user_agent(Some(config.user_agent.clone()))
        .set_wait_for_idle_network(Some(WaitForIdleNetwork::new(Some(Duration::from_secs(10)))))
        .set_wait_for_selector(Some(WaitForSelector::new(Some(Duration::from_secs(5)), "body".into())));

    // Chrome渲染配置
    let chrome_config = if config.crawl.use_chrome {
        Some(
            spider::chrome::ChromeConfig::builder()
                .headless(config.chrome.headless)
                .headless_new(true)
                .timeout(Duration::from_secs(30))
                .proxy(config.proxy.clone().filter(|s| !s.is_empty()))
                .build()?,
        )
    } else {
        None
    };

    // 构建带登录态的Website
    let website = WebsiteBuilder::new(&config.crawl.target_url)
        .with_config(crawler_config)
        .with_chrome_config(chrome_config)
        .with_cookies(Some(cookies))
        .build()
        .context("Failed to build authed Website")?;

    Ok(website)
}
