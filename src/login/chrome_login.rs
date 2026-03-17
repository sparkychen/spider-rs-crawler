use anyhow::{Context, Result};
use cookie::Cookie as ExternalCookie;
use crate::Config;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{info, warn};
use reqwest::{
    Client,
    header::{self, SET_COOKIE},
    redirect::Policy,
};

// ====================== 对外入口：真实网络登录 ======================
pub async fn login_with_chrome(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    // 基础校验
    if config.login.url.is_empty() {
        return Err(anyhow::anyhow!("login.url 不能为空"));
    }
    if config.login.username.is_empty() || config.login.password.is_empty() {
        return Err(anyhow::anyhow!("用户名/密码不能为空"));
    }

    info!(
        "🔌 开始【真实网络登录】 | 登录页: {} | 用户: {}",
        config.login.url, config.login.username
    );

    // 分发：163 走专属流程，其他走通用流程
    let cookies = if config.login.url.contains("163.com") {
        login_netease_real(config).await?
    } else {
        login_generic_real(config).await?
    };

    info!("✅ 真实登录完成，共获取 {} 个 Cookie", cookies.len());
    Ok(cookies)
}

// ====================== 工具：创建模拟浏览器的真实客户端 ======================
fn build_browser_client() -> Result<Client> {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(Policy::limited(5))    
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/128.0.0.0 Safari/537.36")
        .default_headers({
            let mut h = header::HeaderMap::new();
            h.insert(header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".parse().unwrap());
            h.insert(header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9".parse().unwrap());
            h.insert(header::UPGRADE_INSECURE_REQUESTS, "1".parse().unwrap());
            h
        })
        .build()
        .context("创建浏览器客户端失败")?;

    Ok(client)
}

// ====================== 工具：从响应中真实提取 Cookie ======================
fn extract_cookies_real(resp: &reqwest::Response, domain: &str) -> Vec<ExternalCookie<'static>> {
    let mut cookies = Vec::new();

    for h in resp.headers().get_all(SET_COOKIE).iter() {
        let s = match h.to_str() {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Ok(mut c) = ExternalCookie::parse(s) {
            c.set_path(c.path().unwrap_or("/").to_string());
            c.set_domain(c.domain().unwrap_or(domain).to_string());
            c.set_expires(Some(OffsetDateTime::now_utc() + time::Duration::days(1)));
            cookies.push(c.into_owned());
        }
    }

    cookies
}

// ====================== 1. 163 邮箱【真实网络登录】专属流程 ======================
async fn login_netease_real(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    info!("📧 使用 163 邮箱真实登录流程");
    let client = build_browser_client()?;

    // 1. 先访问登录页，拿到基础 Cookie
    let _login_page = client
        .get(&config.login.url)
        .header("Referer", "https://www.163.com/")
        .send()
        .await
        .context("访问 163 登录页失败（网络/反爬/IP 问题）")?;

    // 2. 163 真实登录接口
    let login_api = "https://dl.reg.163.com/sso/login";

    // 纯 String 类型表单（无引用、无不稳定方法）
    let login_form = vec![
        ("username".to_string(), config.login.username.clone()),
        ("password".to_string(), config.login.password.clone()),
        ("product".to_string(), "mail163".to_string()),
        ("type".to_string(), "username".to_string()),
    ];

    // 发送登录请求
    let _login_resp = client
        .post(login_api)
        .form(&login_form)
        .send()
        .await
        .context("发送 163 登录请求失败")?;

    // 3. 验证是否登录成功：尝试跳转到邮箱主页
    let dashboard = client
        .get(&config.crawl.target_url)
        .send()
        .await
        .context("访问邮箱主页失败")?;

    let final_url = dashboard.url().as_str();
    let status = dashboard.status().as_u16();

    if status != 200 || final_url.contains("login") || final_url.contains("reg") {
        return Err(anyhow::anyhow!(
            "163 真实登录失败 → 账号密码错误/需验证码/IP 受限 | 最终地址: {final_url}"
        ));
    }

    // 4. 提取邮箱主页的Cookie（移除多余的mut）
    let cookies = extract_cookies_real(&dashboard, "163.com");
    
    if cookies.is_empty() {
        warn!("⚠️ 未拿到有效 Cookie，但登录页访问正常");
    }

    Ok(cookies)
}

// ====================== 2. 任意网站【通用真实登录】流程 ======================
async fn login_generic_real(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    info!("🌐 使用通用网站真实登录流程（POST 表单）");
    let client = build_browser_client()?;

    // 1. 先访问登录页获取 Cookie
    let _login_page = client
        .get(&config.login.url)
        .send()
        .await
        .context("访问目标登录页失败")?;

    // 纯 String 类型表单
    let login_form = vec![
        ("username".to_string(), config.login.username.clone()),
        ("password".to_string(), config.login.password.clone()),
    ];

    // 发送登录请求
    let _login_resp = client
        .post(&config.login.url)
        .form(&login_form)
        .send()
        .await
        .context("发送通用登录请求失败")?;

    // 2. 验证登录：访问目标页面
    let test = client.get(&config.crawl.target_url).send().await?;
    let final_url = test.url().as_str();

    if final_url.contains("login") || final_url.contains("signin") || test.status().as_u16() == 401 {
        return Err(anyhow::anyhow!("通用网站登录失败：账号密码错误或权限不足"));
    }

    // 3. 提取 Cookie
    let domain = extract_domain(&config.login.url);
    let cookies = extract_cookies_real(&test, &domain);

    Ok(cookies)
}

// ====================== 工具：从 URL 中提取域名 ======================
fn extract_domain(url: &str) -> String {
    let u = url.replace("https://", "").replace("http://", "");
    let parts: Vec<&str> = u.split('/').collect();
    parts[0].to_string()
}
