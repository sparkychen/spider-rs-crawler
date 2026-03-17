use anyhow::{anyhow, Context, Error, Result};
use cookie::Cookie as ExternalCookie;
use headless_chrome::{Browser, Element, LaunchOptions, Tab};
use crate::Config;
use std::{
    path::Path,
    time::{Duration, SystemTime},
};
use time::OffsetDateTime;
use tracing::info;

// ====================== 对外入口：真实浏览器登录 ======================
pub async fn login_with_chrome(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    // 基础校验
    if config.login.url.is_empty() {
        return Err(anyhow!("login.url 不能为空"));
    }

    info!(
        "🔌 开始【真实浏览器登录】 | 登录页: {} ",
        config.login.url
    );

    // 分发：163 走自动验证流程，其他走通用流程
    let cookies = if config.login.url.contains("163.com") {
        login_netease_auto(config).await
            .with_context(|| "163邮箱登录失败")?
    } else {
        login_generic_browser(config).await
            .with_context(|| "通用网站登录失败")?
    };

    info!("✅ 真实浏览器登录完成，共获取 {} 个 Cookie", cookies.len());
    Ok(cookies)
}

// ====================== 启动Chrome浏览器（延长生命周期） ======================
fn launch_browser() -> Result<Browser> {
    let args_str = vec![
        "--no-sandbox",
        "--disable-blink-features=AutomationControlled",
        "--disable-web-security",
        "--ignore-certificate-errors",
        "--disable-application-cache",
        "--disable-default-apps",
        "--disable-popup-blocking",
        "--disable-background-timer-throttling", // 禁用后台定时器节流
        "--disable-renderer-backgrounding",      // 禁用渲染进程后台化
    ];
    let args: Vec<_> = args_str
        .iter()
        .map(|s| Path::new(s).as_os_str())
        .collect();

    let browser = Browser::new(LaunchOptions {
        headless: false, // 显示浏览器，方便手动操作
        args,
        ..LaunchOptions::default()
    })?;
    info!("✅ Chrome浏览器启动成功");
    Ok(browser)
}

// ====================== 等待元素加载（适配1.0.21 API） ======================
async fn wait_for_element<'a>(
    tab: &'a Tab,
    selector: &str,
    timeout: Duration,
) -> Result<Element<'a>> {
    let start = SystemTime::now();
    loop {
        if start.elapsed()?.as_secs() > timeout.as_secs() {
            return Err(anyhow!("等待元素超时：{}", selector));
        }

        match tab.find_element(selector) {
            Ok(elem) => return Ok(elem),
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
        }
    }
}

// ====================== 163邮箱登录（修复初始页面误判失败） ======================
async fn login_netease_auto(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    info!("📧 163邮箱登录流程（自动验证模式）");
    let browser = launch_browser()?;
    let tab = browser.new_tab()?;

    // 1. 打开163登录页
    let login_url = if config.login.url.contains("mail.163.com") {
        "https://mail.163.com/"
    } else {
        config.login.url.as_str()
    };
    tab.navigate_to(login_url)?;
    // 延长初始等待时间，确保登录页完全加载（10秒）
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    info!("✅ 已打开163邮箱登录页（初始操作窗口期20秒），请在页面中完成以下操作：");
    info!("   1. 输入账号：{}", config.login.username);
    info!("   2. 输入密码并完成验证码/滑块验证");
    info!("   3. 点击登录按钮（操作完成后可关闭页面）");
    info!("⚠️  初始窗口期20秒内不会判定失败，请尽快输入账号密码！");
    info!("⚠️  总操作时间3分钟，超时将判定为登录失败！");

    // 2. 自动等待并验证登录状态（核心修复：增加初始窗口期）
    let start_time = SystemTime::now();
    const MAX_WAIT_SECONDS: u64 = 180;       // 总超时时间：3分钟
    const INITIAL_WINDOW_SECONDS: u64 = 20;  // 初始操作窗口期：20秒（这段时间不判定失败）
    let mut last_prompt_time = start_time;
    let mut _login_success = false;

    loop {
        let elapsed = start_time.elapsed()?.as_secs();
        let remaining = MAX_WAIT_SECONDS - elapsed;

        // 总超时判断：直接判定登录失败
        if elapsed >= MAX_WAIT_SECONDS {
            return Err(anyhow!(
                "登录失败：超出最大等待时间（{}秒），请检查账号密码或重新运行程序",
                MAX_WAIT_SECONDS
            ));
        }

        // 每20秒提示一次剩余时间（减少频繁提示）
        if last_prompt_time.elapsed()?.as_secs() >= 20 {
            info!("⏳ 剩余登录时间：{}秒（初始窗口期已过，未登录成功将判定失败）", remaining);
            last_prompt_time = SystemTime::now();
        }

        // 获取当前URL并判断登录状态
        let current_url = tab.get_url();
        info!("🔍 自动检测 - 当前页面URL：{}", current_url);

        // 登录成功判定条件（必须包含main.jsp）
        let is_login_succeeded = current_url.contains("mail.163.com") && current_url.contains("main.jsp");

        // 登录失败判定条件（仅在初始窗口期过后生效）
        let is_login_failed = elapsed > INITIAL_WINDOW_SECONDS && (
            // 1. 仍停留在登录页（无main.jsp）
            (current_url.contains("mail.163.com") && !current_url.contains("main.jsp")) ||
            // 2. URL包含错误标识（账号/密码错误）
            current_url.contains("error") ||
            current_url.contains("wrong") ||
            current_url.contains("fail") ||
            // 3. 页面被关闭（URL为空或about:blank）
            current_url.is_empty() ||
            current_url == "about:blank"
        );

        // 自动判定逻辑（核心修复）
        if is_login_succeeded {
            // 登录成功：立即退出循环
            _login_success = true;
            break;
        } else if is_login_failed {
            // 仅在初始窗口期过后才判定失败
            return Err(anyhow!(
                "登录失败：账号密码错误、验证未通过或页面已关闭（当前URL：{}）",
                current_url
            ));
        } else {
            // 初始窗口期内：仅等待，不判定失败
            if elapsed <= INITIAL_WINDOW_SECONDS {
                info!("ℹ️  初始操作窗口期（剩余{}秒），继续等待登录操作...", INITIAL_WINDOW_SECONDS - elapsed);
            }
        }

        // 检测间隔：3秒（减少CPU占用，给用户足够操作时间）
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    // 3. 登录成功，提取Cookie
    info!("✅ 自动验证通过：163邮箱登录成功！");
    info!("🔍 开始提取登录Cookie...");

    let browser_cookies = tab.get_cookies().with_context(|| {
        "提取163邮箱Cookie失败，请检查浏览器状态"
    })?;

    // 过滤163相关有效Cookie
    let mut valid_cookies = Vec::new();
    for c in browser_cookies {
        if c.domain.contains("163.com") || c.domain.contains("163.net") {
            let cookie_name = c.name.clone();
            let mut cookie = ExternalCookie::new(c.name, c.value);
            
            cookie.set_domain(c.domain);
            cookie.set_path(c.path);
            
            // 处理时间戳异常
            cookie.set_expires(match OffsetDateTime::from_unix_timestamp(c.expires as i64) {
                Ok(dt) => Some(dt),
                Err(_) => {
                    info!("⚠️ Cookie {} 过期时间转换失败，使用当前时间+1天", cookie_name);
                    Some(OffsetDateTime::now_utc() + Duration::from_secs(86400))
                }
            });
            
            cookie.set_secure(c.secure);
            cookie.set_http_only(c.http_only);
            valid_cookies.push(cookie.into_owned());
        }
    }

    // 无有效Cookie → 判定登录失败
    if valid_cookies.is_empty() {
        return Err(anyhow!("登录失败：未提取到163邮箱有效Cookie，账号验证未通过"));
    }

    info!("✅ Cookie提取完成！共获取 {} 个163邮箱有效Cookie", valid_cookies.len());
    
    // 延长浏览器保留时间（10秒），确保Cookie完全生效
    info!("ℹ️  登录验证完成，浏览器将在10秒后自动关闭...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    Ok(valid_cookies)
}

// ====================== 通用网站登录流程（保持不变） ======================
async fn login_generic_browser(config: &Config) -> Result<Vec<ExternalCookie<'static>>> {
    info!("🌐 通用网站登录流程（自动验证模式）");
    let browser = launch_browser()?;
    let tab = browser.new_tab()?;

    // 1. 打开登录页
    tab.navigate_to(&config.login.url)?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    info!("✅ 已打开通用网站登录页，请手动完成登录操作（3分钟超时）");

    // 2. 尝试自动填充账号密码
    if !config.login.username.is_empty() {
        // 优先尝试name=username
        let username_filled = match wait_for_element(&tab, "input[name='username']", Duration::from_secs(5)).await {
            Ok(elem) => {
                elem.click()?;
                elem.type_into(&config.login.username)?;
                info!("✅ 已自动填充账号：{}", config.login.username);
                true
            }
            Err(_) => {
                // 失败则尝试id=username
                match wait_for_element(&tab, "input[id='username']", Duration::from_secs(5)).await {
                    Ok(elem) => {
                        elem.click()?;
                        elem.type_into(&config.login.username)?;
                        info!("✅ 已自动填充账号：{}", config.login.username);
                        true
                    }
                    Err(e) => {
                        info!("⚠️ 自动填充账号失败：{}，请手动输入", e);
                        false
                    }
                }
            }
        };

        // 尝试自动填充密码（添加显式类型注解）
        if !config.login.password.is_empty() && username_filled {
            let _: Result<(), Error> = match wait_for_element(&tab, "input[name='password']", Duration::from_secs(5)).await {
                Ok(elem) => {
                    elem.click()?;
                    elem.type_into(&config.login.password)?;
                    info!("✅ 已自动填充密码（已隐藏）");
                    Ok(())
                }
                Err(_) => {
                    match wait_for_element(&tab, "input[id='password']", Duration::from_secs(5)).await {
                        Ok(elem) => {
                            elem.click()?;
                            elem.type_into(&config.login.password)?;
                            info!("✅ 已自动填充密码（已隐藏）");
                            Ok(())
                        }
                        Err(e) => {
                            info!("⚠️ 自动填充密码失败：{}，请手动输入", e);
                            Ok(())
                        }
                    }
                }
            };
        }
    }

    // 3. 自动等待并验证登录状态
    let start = SystemTime::now();
    let mut login_success = false;
    while start.elapsed()?.as_secs() < 180 {
        let current_url = tab.get_url();
        
        // 自动判定：离开登录页且无登录关键词 → 登录成功
        let is_login_page = current_url.contains("login") || current_url.contains("signin");
        let has_navigated_away = !current_url.eq(&config.login.url);

        if !is_login_page && has_navigated_away {
            login_success = true;
            break;
        }
        
        // 检测失败状态
        let is_failed = current_url.contains("error") || current_url.contains("fail") || current_url.is_empty();
        if is_failed {
            return Err(anyhow!("登录失败：通用网站验证未通过（URL：{}）", current_url));
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    if !login_success {
        return Err(anyhow!("登录失败：通用网站3分钟内未完成登录操作"));
    }
    info!("✅ 自动验证通过：通用网站登录成功！");

    // 4. 提取Cookie
    let browser_cookies = tab.get_cookies()
        .with_context(|| "提取通用网站Cookie失败")?;
    let mut cookies = Vec::new();
    for c in browser_cookies {
        let cookie_name = c.name.clone();
        let mut cookie = ExternalCookie::new(c.name, c.value);
        
        cookie.set_domain(c.domain);
        cookie.set_path(c.path);
        cookie.set_expires(Some(OffsetDateTime::from_unix_timestamp(
            c.expires as i64
        ).unwrap_or_else(|_| {
            info!("⚠️ Cookie {} 过期时间转换失败，使用当前时间", cookie_name);
            OffsetDateTime::now_utc()
        })));
        
        cookie.set_secure(c.secure);
        cookie.set_http_only(c.http_only);
        cookies.push(cookie.into_owned());
    }

    // 自动关闭浏览器
    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(cookies)
}
