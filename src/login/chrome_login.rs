use anyhow::{anyhow, Context, Result};
use cookie::Cookie as ExternalCookie;
// 仅保留1.0.21实际用到的导入，消除警告
use headless_chrome::{Browser, LaunchOptions};
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

// ====================== 启动浏览器（适配1.0.21 + 规避反爬） ======================
fn launch_browser() -> Result<Browser> {
    let args_str = vec![
        "--no-sandbox",
        "--disable-blink-features=AutomationControlled",
        "--disable-web-security",
        "--ignore-certificate-errors",
        "--window-size=1920,1080",
        "--disable-dev-shm-usage",
        "--user-agent=Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
        "--disable-features=VizDisplayCompositor",
        "--lang=zh-CN,zh;q=0.9,en;q=0.8",
        "--enable-automation=false",
        "--disable-extensions",
        "--no-first-run",
        "--no-default-browser-check",
    ];

    let args: Vec<_> = args_str
        .iter()
        .map(|s| Path::new(s).as_os_str())
        .collect();

    let browser = Browser::new(LaunchOptions {
        headless: false, // 可视化模式，便于调试
        args,
        sandbox: false,
        devtools: false,
        enable_gpu: true,
        enable_logging: true,
        window_size: Some((1920, 1080)),
        ..LaunchOptions::default()
    })?;

    info!("✅ Chrome浏览器（可视化模式）启动成功");
    Ok(browser)
}

// ====================== 核心修复：适配163登录页 + 1.0.21 API ======================
async fn login_netease_auto(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    let browser = launch_browser()?;
    let tab = browser.new_tab()?;

    // 1. 访问163邮箱主站
    info!("🌐 访问163邮箱主站：https://mail.163.com/");
    tab.navigate_to("https://mail.163.com/")?;
    std::thread::sleep(Duration::from_secs(8));
    info!("📌 当前页面URL：{}", tab.get_url());

    // 2. 修复：1.0.21无get_attribute，用get_attributes解析iframe的src
    info!("🔍 查找登录iframe");
    let iframe_src = match tab.wait_for_element("iframe[id='x-URS-iframe']") {
        Ok(iframe_elem) => {
            // 1.0.21的get_attributes返回Option<Vec<String>>，格式为["key1", "value1", "key2", "value2"]
            let attrs = iframe_elem.get_attributes()?.unwrap_or_default();
            let mut src = String::new();
            // 遍历属性列表，找到src属性
            for i in (0..attrs.len()).step_by(2) {
                if attrs[i] == "src" && i+1 < attrs.len() {
                    src = attrs[i+1].clone();
                    break;
                }
            }
            info!("🔍 登录iframe真实地址：{}", src);
            src
        },
        Err(e) => {
            return Err(anyhow!("未找到登录iframe：{}", e));
        }
    };

    // 3. 直接访问iframe地址（绕过嵌套问题）
    if !iframe_src.is_empty() {
        info!("🌐 直接访问登录iframe地址：{}", iframe_src);
        tab.navigate_to(&iframe_src)?;
        std::thread::sleep(Duration::from_secs(8));
    } else {
        return Err(anyhow!("获取iframe地址失败"));
    }

    // 4. 定位账号输入框（1.0.21兼容）
    info!("🔍 开始定位账号输入框");
    let user_input = match tab.wait_for_element("input[name='email']") {
        Ok(elem) => {
            info!("✅ 找到账号输入框（name=email）");
            elem
        },
        Err(_) => match tab.wait_for_element("input[id='username']") {
            Ok(elem) => {
                info!("✅ 找到账号输入框（id=username）");
                elem
            },
            Err(e) => {
                let html = tab.get_content()?;
                info!("❌ 账号输入框定位失败，页面源码片段：{}", &html[0..500]);
                return Err(anyhow!("账号输入框定位失败：{}", e));
            }
        }
    };

    // 5. 输入账号（修复：1.0.21无press_key，直接type_into覆盖）
    info!("✏️ 开始输入账号：{}", config.login.username);
    user_input.click()?;
    std::thread::sleep(Duration::from_millis(1000));
    // 直接输入空字符串清空，再输入账号（替代press_key全选删除）
    user_input.type_into("")?;
    std::thread::sleep(Duration::from_millis(500));
    user_input.type_into(&config.login.username)?;
    std::thread::sleep(Duration::from_secs(2));
    info!("✅ 账号输入完成");

    // 6. 定位并输入密码（1.0.21兼容）
    info!("🔍 开始定位密码输入框");
    let pwd_input = match tab.wait_for_element("input[name='password']") {
        Ok(elem) => {
            info!("✅ 找到密码输入框（name=password）");
            elem
        },
        Err(_) => match tab.wait_for_element("input[id='password']") {
            Ok(elem) => {
                info!("✅ 找到密码输入框（id=password）");
                elem
            },
            Err(e) => {
                let html = tab.get_content()?;
                info!("❌ 密码输入框定位失败，页面源码片段：{}", &html[0..500]);
                return Err(anyhow!("密码输入框定位失败：{}", e));
            }
        }
    };

    info!("🔑 开始输入密码");
    pwd_input.click()?;
    std::thread::sleep(Duration::from_millis(1000));
    // 直接输入空字符串清空，再输入密码
    pwd_input.type_into("")?;
    std::thread::sleep(Duration::from_millis(500));
    pwd_input.type_into(&config.login.password)?;
    std::thread::sleep(Duration::from_secs(2));
    info!("✅ 密码输入完成");

    // 7. 定位并点击登录按钮
    info!("🔍 开始定位登录按钮");
    let login_btn = match tab.wait_for_element("input[id='dologin']") {
        Ok(elem) => {
            info!("✅ 找到登录按钮（id=dologin）");
            elem
        },
        Err(_) => match tab.wait_for_element("button[class*='login-btn']") {
            Ok(elem) => {
                info!("✅ 找到登录按钮（class=login-btn）");
                elem
            },
            Err(e) => {
                let html = tab.get_content()?;
                info!("❌ 登录按钮定位失败，页面源码片段：{}", &html[0..500]);
                return Err(anyhow!("登录按钮定位失败：{}", e));
            }
        }
    };

    info!("🚀 点击登录按钮");
    login_btn.click()?;
    std::thread::sleep(Duration::from_secs(20));
    info!("✅ 登录按钮点击完成，当前URL：{}", tab.get_url());

    // 8. 验证登录状态
    let current_url = tab.get_url();
    let login_success = current_url.contains("main.jsp") 
        || current_url.contains("mail163.com") 
        || (current_url.contains("https://mail.163.com/") && !current_url.contains("login"));

    if !login_success {
        // 检测验证码/安全验证
        let html = tab.get_content()?;
        let has_captcha = html.contains("验证码") || html.contains("安全验证");
        let error_msg = if has_captcha {
            "检测到验证码/安全验证（需手动关闭账号安全保护）".to_string()
        } else if html.contains("密码错误") {
            "账号或密码错误".to_string()
        } else {
            "未触发登录（元素定位/点击失败）".to_string()
        };

        return Err(anyhow!(
            "163邮箱登录失败：\n错误原因：{}\n当前URL：{}\n排查步骤：\n1. 手动登录https://mail.163.com，关闭所有安全验证\n2. 确保IP为账号常用登录地\n3. 观察浏览器窗口，确认输入框/按钮是否正确点击",
            error_msg, current_url
        ));
    }

    // 9. 提取Cookie
    info!("📥 开始提取Cookie");
    let browser_cookies = tab.get_cookies()?;
    let mut valid_cookies = Vec::new();

    for c in browser_cookies {
        if c.domain.contains("163.com") || c.domain.contains("netease.com") {
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
        return Err(anyhow!("登录成功但未提取到有效Cookie"));
    }

    info!("✅ 163邮箱登录成功，获取到 {} 个有效Cookie", valid_cookies.len());
    Ok(valid_cookies)
}

// ====================== 通用网站登录（适配1.0.21） ======================
async fn login_generic_browser(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    info!("🌐 通用网站登录（无界面自动填充）");
    let browser = launch_browser()?;
    let tab = browser.new_tab()?;

    tab.navigate_to(&config.login.url)?;
    tab.wait_for_element("html")?;
    std::thread::sleep(Duration::from_secs(5));

    if !config.login.username.is_empty() {
        info!("✏️ 自动填充账号：{}", config.login.username);
        if let Ok(elem) = tab.wait_for_element("input[name='username']") {
            elem.click()?;
            std::thread::sleep(Duration::from_millis(500));
            elem.type_into(&config.login.username)?;
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    if !config.login.password.is_empty() {
        info!("🔑 自动填充密码");
        if let Ok(elem) = tab.wait_for_element("input[name='password']") {
            elem.click()?;
            std::thread::sleep(Duration::from_millis(500));
            elem.type_into(&config.login.password)?;
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    if let Ok(elem) = tab.wait_for_element("button[type='submit']") {
        elem.click()?;
        std::thread::sleep(Duration::from_secs(10));
    }

    let current_url = tab.get_url();
    let is_login_page = current_url.contains("login") || current_url.contains("signin");
    let has_navigated_away = !current_url.eq(&config.login.url);

    if is_login_page && !has_navigated_away {
        return Err(anyhow!("登录失败：通用网站无界面登录未跳转"));
    }

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
