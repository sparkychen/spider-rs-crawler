use anyhow::{Context, Result};
use redis::{Client, cmd};
use cookie::Cookie as ExternalCookie;
use std::time::{SystemTime, Duration};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct CookieStore {
    client: Client,
}

impl CookieStore {
    /// 初始化Redis客户端
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url)
            .context(format!("❌ 连接Redis失败: {}", redis_url))?;
        
        let mut conn = client.get_multiplexed_async_connection().await?;
        let pong: String = cmd("PING").query_async(&mut conn).await?;
        if pong != "PONG" {
            return Err(anyhow::anyhow!("❌ Redis Ping失败: {}", pong));
        }

        Ok(Self { client })
    }

    /// 检查Cookie是否有效
    pub async fn is_cookie_valid(&self, key: &str) -> Result<bool> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let ttl: i64 = cmd("TTL").arg(key).query_async(&mut conn).await?;
        Ok(ttl > 0)
    }

    /// 读取Cookie（修复生命周期错误：将&str转为String，让Cookie拥有所有权）
    pub async fn get_cookies(&self, key: &str) -> Result<Vec<ExternalCookie<'static>>> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let cookie_str: String = match cmd("GET").arg(key).query_async(&mut conn).await {
            Ok(val) => val,
            Err(_) => return Ok(Vec::new()),
        };
        
        let mut cookies = Vec::new();
        for s in cookie_str.split("; ") {
            let parts: Vec<&str> = s.splitn(2, '=').collect();
            if parts.len() == 2 {
                // ✅ 关键修复：将&str转为String，避免引用本地变量
                let name = parts[0].to_string();
                let value = parts[1].to_string();
                
                // 构造拥有所有权的Cookie（无引用，生命周期'static）
                let mut cookie = ExternalCookie::new(name, value);
                cookie.set_domain("example.com".to_string());
                cookie.set_path("/".to_string());
                
                // 时间转换（无变化）
                let expires = OffsetDateTime::from(SystemTime::now() + Duration::from_secs(3600));
                cookie.set_expires(Some(expires));
                
                cookies.push(cookie);
            }
        }

        Ok(cookies)
    }

    /// 存储Cookie
    #[allow(dead_code)] // ✅ 新增：忽略未使用警告
    pub async fn set_cookies(&self, key: &str, cookies: &[ExternalCookie<'_>], ttl: u64) -> Result<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let cookie_str = cookies.iter()
            .map(|c| format!("{}={}", c.name(), c.value()))
            .collect::<Vec<_>>()
            .join("; ");
        
        cmd("SETEX")
            .arg(key)
            .arg(ttl)
            .arg(cookie_str)
            .query_async::<_, ()>(&mut conn)
            .await?;
        
        Ok(())
    }
}
