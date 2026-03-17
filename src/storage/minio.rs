use anyhow::{Context, Result};
use minio::s3::client::Client;
use minio::s3::creds::StaticProvider;
use std::io::Cursor;
use tokio::task; // 用于异步包装同步调用
use crate::config::StorageConfig;

/// 保存HTML到MinIO（适配0.3.0同步API，包装为异步）
pub async fn save_html(config: &StorageConfig, url: &str, html: String) -> Result<()> {
    // 复制配置到闭包（避免生命周期问题）
    let minio_config = config.minio.clone();
    let url = url.to_string();
    let html = html;

    // 用tokio::spawn_blocking包装同步MinIO调用
    let result = task::spawn_blocking(move || -> Result<()> {
        // 构建MinIO客户端（同步）
        let provider = StaticProvider::new(
            minio_config.access_key,
            minio_config.secret_key,
            None,
        );

        let client = Client::new(&minio_config.endpoint, provider)
            .context("Failed to create MinIO client")?;

        // 检查桶是否存在
        let bucket_exists = client.bucket_exists(&minio_config.bucket)
            .context("Failed to check bucket existence")?;
        if !bucket_exists {
            client.make_bucket(&minio_config.bucket, None)
                .context("Failed to create bucket")?;
            tracing::info!("Created MinIO bucket: {}", minio_config.bucket);
        }

        // 生成对象名
        let object_name = format!("{}.html", url.replace('/', "_").replace(":", "_").replace("?", "_"));
        let content = html.into_bytes();
        let cursor = Cursor::new(content.clone());

        // 同步上传文件
        client.put_object(
            &minio_config.bucket,
            &object_name,
            cursor,
            content.len() as u64,
            None,
        ).context(format!("Failed to upload {} to MinIO", object_name))?;

        tracing::debug!("Saved HTML to MinIO: {}", object_name);
        Ok(())
    }).await?; // 等待同步任务完成

    result
}
