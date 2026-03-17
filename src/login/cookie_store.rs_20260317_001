use anyhow::{Context, Result};
use redis::Client;
use spider::cookie::Cookie;
use std::time::Duration;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct StoredCookie {
    name: String,
    value: String,
    domain: Option<String>,
    path: Option<String>,
    expires: Option<i64>,
    secure: bool,
    http_only: bool,
}

impl From<&Cookie> for StoredCookie {
    fn from(cookie: &Cookie) -> Self {
        Self {
            name: cookie.name().to_string(),
            value: cookie.value().to_string(),
            domain: cookie.domain().map(|d| d.to_string()),
            path: cookie.path().map(|p| p.to_string()),
            expires: cookie.expires().map(|t| t.timestamp()),
            secure: cookie.secure(),
            http_only: cookie.http_only(),
        }
    }
}

impl From<&StoredCookie> for Cookie {
    fn from(sc: &StoredCookie) -> Self {
        let mut cookie = Cookie::new(sc.name.clone(), sc.value.clone());
        if let Some(domain) = &sc.domain {
            cookie.set_domain(domain.clone());
        }
        if let Some(path) = &sc.path {
            cookie.set_path(path.clone());
        }
        cookie.set_secure(sc.secure);
        cookie.set_http_only(sc.http_only);
        cookie
    }
}

pub struct CookieStore {
    client: Client,
}

impl CookieStore {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url).context("Failed to connect to Redis")?;
        Ok(Self { client })
    }

    pub async fn set_cookies(&self, key: &str, cookies: &[Cookie], ttl: Duration) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        let stored: Vec<StoredCookie> = cookies.iter().map(StoredCookie::from).collect();
        let json = serde_json::to_string(&stored)?;
        redis::cmd("SETEX")
            .arg(key)
            .arg(ttl.as_secs())
            .arg(json)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    pub async fn get_cookies(&self, key: &str) -> Result<Vec<Cookie>> {
        let mut conn = self.client.get_async_connection().await?;
        let json: Option<String> = redis::cmd("GET").arg(key).query_async(&mut conn).await?;
        let json = json.context("Cookie not found")?;
        let stored: Vec<StoredCookie> = serde_json::from_str(&json)?;
        Ok(stored.iter().map(Cookie::from).collect())
    }

    pub async fn is_cookie_valid(&self, key: &str) -> Result<bool> {
        let mut conn = self.client.get_async_connection().await?;
        let ttl: i64 = redis::cmd("TTL").arg(key).query_async(&mut conn).await?;
        Ok(ttl > 60) // 剩余有效期大于60秒视为有效
    }

    pub async fn delete_cookie(&self, key: &str) -> Result<()> {
        let mut conn = self.client.get_async_connection().await?;
        redis::cmd("DEL").arg(key).query_async(&mut conn).await?;
        Ok(())
    }
}
