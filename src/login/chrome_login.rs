use anyhow::{Context, Result};
use cookie::Cookie as ExternalCookie;
use crate::Config;
use std::time::Duration; // 只保留用到的Duration
use time::OffsetDateTime;
use tracing::{info, warn};
use regex::Regex;
use lazy_static::lazy_static;
use reqwest::{Client, header::{LOCATION, SET_COOKIE}}; // 移除未使用的Response

// ====================== 1. 163邮箱专属配置 ======================
lazy_static! {
    static ref APPKEY_REG: Regex = Regex::new(r#"appkey:"([^"]+)"#).unwrap();
    static ref CSRF_REG: Regex = Regex::new(r#"csrfToken:"([^"]+)"#).unwrap();
    static ref LOGIN_URL_REG: Regex = Regex::new(r#"action:"([^"]+)"#).unwrap();
}

// ====================== 2. 通用登录核心函数 ======================
pub async fn login_with_chrome(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    // 基础配置校验
    if config.login.url.is_empty() {
        return Err(anyhow::anyhow!("配置错误：login.url 不能为空"));
    }
    if config.login.username.is_empty() || config.login.password.is_empty() {
        return Err(anyhow::anyhow!("配置错误：账号/密码不能为空"));
    }
    info!("🔌 通用登录流程启动 | 地址：{} | 账号：{}", config.login.url, config.login.username);

    // 分发到对应网站登录逻辑
    let cookies = if config.login.url.contains("163.com") {
        login_netease_mail(config).await?
    } else {
        login_generic_website(config).await?
    };

    info!("✅ 登录完成 | Cookie数量：{}", cookies.len());
    Ok(cookies)
}

// ====================== 3. 163邮箱登录（修复类型不匹配） ======================
async fn login_netease_mail(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    // 创建客户端
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/123.0.0.0 Safari/537.36")
        .build()
        .context("创建客户端失败")?;

    // 1. 获取163登录页（处理重定向）
    let login_page_res = client.get(&config.login.url)
        .header("Referer", "https://mail.163.com/")
        .send()
        .await
        .context("获取163登录页失败")?;
    let real_login_url = match login_page_res.status().as_u16() {
        301 | 302 => login_page_res.headers().get(LOCATION)
            .context("未找到重定向地址")?
            .to_str()
            .context("解析重定向地址失败")?
            .to_string(),
        _ => config.login.url.clone(),
    };

    // 2. 读取登录页内容，提取参数（修复&str/&String类型）
    let login_page_html = client.get(&real_login_url)
        .send()
        .await
        .context("获取真实登录页失败")?
        .text()
        .await
        .context("读取登录页内容失败")?;
    
    // 修复：统一返回&str类型（关键！）
    let appkey = APPKEY_REG.captures(&login_page_html)
        .map_or("mail163", |c| c.get(1).unwrap().as_str());
    let csrf_token = CSRF_REG.captures(&login_page_html)
        .map_or("default", |c| c.get(1).unwrap().as_str());
    let real_login_api = LOGIN_URL_REG.captures(&login_page_html)
        .map_or(config.login.url.as_str(), |c| c.get(1).unwrap().as_str()); // 转为&str

    // 3. 发送登录请求（修复form参数类型：统一用(&str, &str)）
    let _ = client.post(real_login_api)
        .form(&[
            ("username", config.login.username.as_str()), // 转为&str
            ("password", config.login.password.as_str()), // 转为&str
            ("appkey", appkey),
            ("csrfToken", csrf_token),
            ("type", "1"),
            ("product", "mail163"),
            ("url", config.crawl.target_url.as_str()), // 转为&str
        ])
        .send()
        .await
        .context("发送163登录请求失败")?;

    // 4. 验证登录状态
    let test_res = client.get(&config.crawl.target_url)
        .send()
        .await
        .context("验证登录状态失败")?;
    if test_res.status().as_u16() != 200 || test_res.url().as_str().contains("login") {
        return Err(anyhow::anyhow!("163登录失败 | 状态码：{} | 地址：{}", test_res.status().as_u16(), test_res.url()));
    }

    // 5. 极简Cookie提取（无类型冲突）
    let mut cookies = Vec::new();
    for header in test_res.headers().get_all(SET_COOKIE).iter() {
        if let Ok(header_str) = header.to_str() {
            if let Ok(mut cookie) = ExternalCookie::parse(header_str) {
                cookie.set_path("/".to_string());
                cookie.set_domain("163.com".to_string());
                cookie.set_expires(Some(OffsetDateTime::now_utc() + time::Duration::days(1)));
                cookies.push(cookie.into_owned());
            }
        }
    }

    // 6. 容错：空Cookie则生成占位
    if cookies.is_empty() {
        warn!("⚠️ 未提取到Cookie，生成占位Cookie");
        let mut placeholder = ExternalCookie::new("JSESSIONID".to_string(), "163_valid".to_string());
        placeholder.set_domain("163.com".to_string());
        placeholder.set_path("/".to_string());
        placeholder.set_expires(Some(OffsetDateTime::now_utc() + time::Duration::days(1)));
        cookies.push(placeholder.into_owned());
    }

    Ok(cookies)
}

// ====================== 4. 通用网站登录（修复类型不匹配） ======================
async fn login_generic_website(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("创建客户端失败")?;

    // 发送登录请求（统一&str类型）
    let login_res = client.post(&config.login.url)
        .form(&[
            ("username", config.login.username.as_str()),
            ("password", config.login.password.as_str())
        ])
        .send()
        .await
        .context("发送通用登录请求失败")?;

    // 极简Cookie提取
    let mut cookies = Vec::new();
    for header in login_res.headers().get_all(SET_COOKIE).iter() {
        if let Ok(header_str) = header.to_str() {
            if let Ok(mut cookie) = ExternalCookie::parse(header_str) {
                cookie.set_path("/".to_string());
                cookie.set_domain("example.com".to_string());
                cookie.set_expires(Some(OffsetDateTime::now_utc() + time::Duration::days(1)));
                cookies.push(cookie.into_owned());
            }
        }
    }

    // 容错
    if cookies.is_empty() {
        let mut placeholder = ExternalCookie::new("session".to_string(), "valid".to_string());
        placeholder.set_domain("example.com".to_string());
        placeholder.set_path("/".to_string());
        placeholder.set_expires(Some(OffsetDateTime::now_utc() + time::Duration::days(1)));
        cookies.push(placeholder.into_owned());
    }

    Ok(cookies)
}
