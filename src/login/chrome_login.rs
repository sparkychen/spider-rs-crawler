use anyhow::{anyhow, Context, Result};
use cookie::Cookie as ExternalCookie;
use headless_chrome::{Browser, Element, LaunchOptions};
use crate::Config;
use std::{
    path::Path,
    time::Duration,
};
use time::OffsetDateTime;
use tracing::info;

// ====================== 对外入口：无界面全自动163邮箱登录 ======================
pub async fn login_with_chrome(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    if config.login.url.is_empty() {
        return Err(anyhow!("login.url 不能为空"));
    }

    info!(
        "🔌 开始【无界面全自动163邮箱登录】 | 配置账号：{}",
        config.login.username
    );

    let cookies = if config.login.url.contains("163.com") {
        login_netease_auto(config).await
            .with_context(|| "163邮箱登录失败")?
    } else {
        login_generic_browser(config).await
            .with_context(|| "通用网站登录失败")?
    };

    info!("✅ 登录完成，共获取 {} 个有效Cookie", cookies.len());
    Ok(cookies)
}

// ====================== 启动浏览器（1.0.21极简兼容 + 无界面） ======================
fn launch_browser() -> Result<Browser> {
    let args_str = vec![
        "--no-sandbox",
        "--disable-blink-features=AutomationControlled",
        "--disable-web-security",
        "--ignore-certificate-errors",
        "--window-size=1920,1080",
        "--disable-dev-shm-usage",
        "--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "--enable-automation=false",
        "--lang=zh-CN",
    ];

    let args: Vec<_> = args_str
        .iter()
        .map(|s| Path::new(s).as_os_str())
        .collect();

    // 核心：无界面模式（不打开任何页面）
    let browser = Browser::new(LaunchOptions {
        headless: true, // 无界面核心配置
        args,
        sandbox: false,
        devtools: false,
        enable_gpu: false,
        enable_logging: false,
        window_size: Some((1920, 1080)),
        ..LaunchOptions::default()
    })?;

    info!("✅ Chrome浏览器（无头模式）启动成功");
    Ok(browser)
}

// ====================== 163邮箱无界面登录（放弃iframe，1.0.21纯原生） ======================
async fn login_netease_auto(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    let browser = launch_browser()?;
    let tab = browser.new_tab()?;

    // 关键：访问163非iframe登录页（核心修复，避免iframe问题）
    info!("🌐 打开163非iframe登录页（无界面）");
    tab.navigate_to("https://mail.163.com/entry/cgi/ntesdoor?iframe=1")?;
    std::thread::sleep(Duration::from_secs(15)); // 延长加载等待

    // 2. 定位账号输入框并自动输入（1.0.21兼容，明确类型注解）
    info!("✏️ 自动填充账号：{}", config.login.username);
    let user_input: Element<'_> = match tab.wait_for_element("input[name='email']") {
        Ok(elem) => elem,
        Err(_) => match tab.wait_for_element("input[id='username']") {
            Ok(elem) => elem,
            Err(e) => return Err(anyhow!("账号输入框定位失败：{}", e)),
        },
    };

    // 无界面自动输入账号（1.0.21兼容）
    let _ = user_input.click();
    std::thread::sleep(Duration::from_millis(800));
    let _ = user_input.type_into(""); // 清空
    std::thread::sleep(Duration::from_millis(500));
    let _ = user_input.type_into(&config.login.username); // 自动输入账号
    std::thread::sleep(Duration::from_secs(1));

    // 3. 定位密码输入框并自动输入（核心：无界面，无需打开页面）
    info!("🔑 自动填充密码（无界面模式）");
    let pwd_input: Element<'_> = match tab.wait_for_element("input[name='password']") {
        Ok(elem) => elem,
        Err(_) => match tab.wait_for_element("input[id='password']") {
            Ok(elem) => elem,
            Err(e) => return Err(anyhow!("密码输入框定位失败：{}", e)),
        },
    };

    // 无界面自动输入密码（1.0.21兼容，全程后台）
    let _ = pwd_input.click();
    std::thread::sleep(Duration::from_millis(800));
    let _ = pwd_input.type_into(""); // 清空
    std::thread::sleep(Duration::from_millis(500));
    let _ = pwd_input.type_into(&config.login.password); // 自动输入密码
    std::thread::sleep(Duration::from_secs(1));

    // 4. 定位并点击登录按钮（1.0.21兼容）
    info!("🚀 点击登录按钮（无界面）");
    let login_btn: Element<'_> = match tab.wait_for_element("input[type='submit']") {
        Ok(elem) => elem,
        Err(_) => match tab.wait_for_element("#dologin") {
            Ok(elem) => elem,
            Err(e) => return Err(anyhow!("登录按钮定位失败：{}", e)),
        },
    };
    let _ = login_btn.click();

    // 5. 等待登录结果（1.0.21兼容）
    info!("⌛ 等待登录结果（20秒，无界面）...");
    std::thread::sleep(Duration::from_secs(20));

    // 6. 验证登录状态（1.0.21兼容）
    let current_url = tab.get_url();
    info!("🔍 登录后URL：{}", current_url);

    // 163登录成功的URL特征
    let login_success = current_url.contains("main.jsp") 
        || current_url.contains("mail163") 
        || !current_url.contains("login") 
        || !current_url.contains("entry");

    if !login_success {
        return Err(anyhow!(
            "无界面登录失败（必做解决方案）：\n1. 账号必须关闭所有安全验证（登录保护/验证码/异地验证）\n2. 确保配置文件账号密码100%正确\n3. 网络环境需稳定（避免异地IP检测）\n当前URL：{}",
            current_url
        ));
    }

    // 7. 提取Cookie（1.0.21兼容）
    info!("📥 提取163邮箱Cookie...");
    let browser_cookies = tab.get_cookies()?;
    let mut valid_cookies = Vec::new();

    for c in browser_cookies {
        if c.domain.contains("163.com") {
            let mut cookie = ExternalCookie::new(c.name, c.value);
            cookie.set_domain(c.domain);
            cookie.set_path(c.path);
            cookie.set_secure(c.secure);
            cookie.set_http_only(c.http_only);
            
            if c.expires > 0.0 {
                if let Ok(dt) = OffsetDateTime::from_unix_timestamp(c.expires as i64) {
                    cookie.set_expires(Some(dt));
                }
            }
            valid_cookies.push(cookie.into_owned());
        }
    }

    if valid_cookies.is_empty() {
        return Err(anyhow!("登录失败：未提取到有效Cookie"));
    }

    info!("✅ 163邮箱无界面登录成功，获取到 {} 个Cookie", valid_cookies.len());
    Ok(valid_cookies)
}

// ====================== 通用网站登录（1.0.21兼容，修复返回值问题） ======================
async fn login_generic_browser(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    info!("🌐 通用网站登录（无界面自动填充）");
    let browser = launch_browser()?;
    let tab = browser.new_tab()?;

    // 打开登录页
    tab.navigate_to(&config.login.url)?;
    std::thread::sleep(Duration::from_secs(10));

    // 自动填充账号（修复返回值引用问题）
    if !config.login.username.is_empty() {
        info!("✏️ 自动填充账号：{}", config.login.username);
        if let Ok(elem) = tab.wait_for_element("input[name='username']") {
            let _ = elem.click();
            std::thread::sleep(Duration::from_millis(500));
            let _ = elem.type_into(&config.login.username);
        }
    }

    // 自动填充密码（修复返回值引用问题）
    if !config.login.password.is_empty() {
        info!("🔑 自动填充密码（无界面）");
        if let Ok(elem) = tab.wait_for_element("input[name='password']") {
            let _ = elem.click();
            std::thread::sleep(Duration::from_millis(500));
            let _ = elem.type_into(&config.login.password);
        }
    }

    // 自动提交（修复返回值引用问题）
    if let Ok(elem) = tab.wait_for_element("button[type='submit']") {
        let _ = elem.click();
        std::thread::sleep(Duration::from_secs(10));
    }

    // 验证登录状态
    let current_url = tab.get_url();
    let is_login_page = current_url.contains("login") || current_url.contains("signin");
    let has_navigated_away = !current_url.eq(&config.login.url);

    if is_login_page && !has_navigated_away {
        return Err(anyhow!("登录失败：通用网站无界面登录未跳转"));
    }

    // 提取Cookie
    info!("📥 提取通用网站Cookie...");
    let browser_cookies = tab.get_cookies()?;
    let mut cookies = Vec::new();

    for c in browser_cookies {
        let mut cookie = ExternalCookie::new(c.name, c.value);
        cookie.set_domain(c.domain);
        cookie.set_path(c.path);
        cookie.set_secure(c.secure);
        cookie.set_http_only(c.http_only);
        
        if c.expires > 0.0 {
            if let Ok(dt) = OffsetDateTime::from_unix_timestamp(c.expires as i64) {
                cookie.set_expires(Some(dt));
            }
        }
        cookies.push(cookie.into_owned());
    }

    info!("✅ 通用网站无界面登录成功，获取到 {} 个Cookie", cookies.len());
    Ok(cookies)
}
