use anyhow::{Context, Result};
use hyper::{Body, Request, Response, Server};
use prometheus::{register_int_counter, register_gauge, IntCounter, Gauge};
use std::net::SocketAddr;
use std::sync::OnceLock;

// 全局指标
static CRAWL_TOTAL: OnceLock<IntCounter> = OnceLock::new();
static CRAWL_SUCCESS: OnceLock<IntCounter> = OnceLock::new();
static CRAWL_FAILED: OnceLock<IntCounter> = OnceLock::new();
static CURRENT_CONCURRENCY: OnceLock<Gauge> = OnceLock::new();

/// 初始化指标
fn init_metrics() {
    CRAWL_TOTAL.get_or_init(|| register_int_counter!("spider_crawl_total", "Total crawl requests").unwrap());
    CRAWL_SUCCESS.get_or_init(|| register_int_counter!("spider_crawl_success", "Successful crawl requests").unwrap());
    CRAWL_FAILED.get_or_init(|| register_int_counter!("spider_crawl_failed", "Failed crawl requests").unwrap());
    CURRENT_CONCURRENCY.get_or_init(|| register_gauge!("spider_current_concurrency", "Current crawl concurrency").unwrap());
}

/// 启动Prometheus指标服务
pub async fn start_metrics_server(port: u16) -> Result<Server<hyper::server::conn::AddrIncoming, hyper::service::make_service_fn<_, _>>> {
    init_metrics();
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let make_svc = hyper::service::make_service_fn(|_conn| async move {
        Ok::<_, hyper::Error>(hyper::service::service_fn(|req: Request<Body>| async move {
            if req.uri().path() == "/metrics" {
                let encoder = prometheus::TextEncoder::new();
                let metric_families = prometheus::gather();
                let mut buffer = Vec::new();
                encoder.encode(&metric_families, &mut buffer).unwrap();
                Ok(Response::new(Body::from(buffer)))
            } else {
                Ok(Response::builder().status(404).body(Body::from("Not Found")).unwrap())
            }
        }))
    });

    let server = Server::bind(&addr).serve(make_svc);
    tracing::info!("Metrics server started on http://0.0.0.0:{}", port);
    Ok(server)
}

/// 记录爬取成功
pub fn inc_crawl_success() {
    CRAWL_TOTAL.get().unwrap().inc();
    CRAWL_SUCCESS.get().unwrap().inc();
}

/// 记录爬取失败
pub fn inc_crawl_failed() {
    CRAWL_TOTAL.get().unwrap().inc();
    CRAWL_FAILED.get().unwrap().inc();
}

/// 获取爬取成功率
pub fn get_success_rate() -> f64 {
    let total = CRAWL_TOTAL.get().unwrap().get();
    if total == 0 {
        return 1.0;
    }
    CRAWL_SUCCESS.get().unwrap().get() as f64 / total as f64
}
