#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use ntex::http::header::{CONTENT_TYPE, SERVER};
use ntex::http::{self, HttpServiceConfig, KeepAlive};
use ntex::util::Bytes;
use ntex::{io::IoConfig, time::Seconds, web, SharedCfg};
use rustls::ServerConfig;
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Arc;

static HDR_SERVER: http::header::HeaderValue = http::header::HeaderValue::from_static("ntex");
static HDR_JSON: http::header::HeaderValue =
    http::header::HeaderValue::from_static("application/json");
static HDR_TEXT: http::header::HeaderValue =
    http::header::HeaderValue::from_static("text/plain");
static BODY_OK: Bytes = Bytes::from_static(b"ok");

#[derive(Deserialize, Clone)]
struct Rating {
    score: f64,
    count: i64,
}

#[derive(Deserialize, Clone)]
struct DatasetItem {
    id: i64,
    name: String,
    category: String,
    price: f64,
    quantity: i64,
    active: bool,
    tags: Vec<String>,
    rating: Rating,
}

#[derive(Serialize)]
struct RatingOut {
    score: f64,
    count: i64,
}

#[derive(Serialize)]
struct ProcessedItem {
    id: i64,
    name: String,
    category: String,
    price: f64,
    quantity: i64,
    active: bool,
    tags: Vec<String>,
    rating: RatingOut,
    total: f64,
}

#[derive(Serialize)]
struct JsonResponse {
    items: Vec<ProcessedItem>,
    count: usize,
}

struct AppState {
    dataset: Vec<DatasetItem>,
}

fn load_dataset() -> Vec<DatasetItem> {
    let path = std::env::var("DATASET_PATH").unwrap_or_else(|_| "/data/dataset.json".to_string());
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn parse_query_sum(query: &str) -> i64 {
    let mut sum: i64 = 0;
    for pair in query.split('&') {
        if let Some(val) = pair.split('=').nth(1) {
            if let Ok(n) = val.parse::<i64>() {
                sum += n;
            }
        }
    }
    sum
}

#[web::get("/pipeline")]
async fn pipeline() -> web::HttpResponse {
    let mut resp =
        web::HttpResponse::with_body(http::StatusCode::OK, http::body::Body::Bytes(BODY_OK.clone()));
    resp.headers_mut().insert(SERVER, HDR_SERVER.clone());
    resp.headers_mut().insert(CONTENT_TYPE, HDR_TEXT.clone());
    resp
}

#[web::get("/baseline11")]
async fn baseline_get(req: web::HttpRequest) -> web::HttpResponse {
    let sum = req
        .uri()
        .query()
        .map(parse_query_sum)
        .unwrap_or(0);
    let body = sum.to_string();
    let mut resp = web::HttpResponse::with_body(http::StatusCode::OK, body.into());
    resp.headers_mut().insert(SERVER, HDR_SERVER.clone());
    resp.headers_mut().insert(CONTENT_TYPE, HDR_TEXT.clone());
    resp
}

#[web::post("/baseline11")]
async fn baseline_post(req: web::HttpRequest, body: Bytes) -> web::HttpResponse {
    let mut sum = req
        .uri()
        .query()
        .map(parse_query_sum)
        .unwrap_or(0);
    if let Ok(s) = std::str::from_utf8(&body) {
        if let Ok(n) = s.trim().parse::<i64>() {
            sum += n;
        }
    }
    let out = sum.to_string();
    let mut resp = web::HttpResponse::with_body(http::StatusCode::OK, out.into());
    resp.headers_mut().insert(SERVER, HDR_SERVER.clone());
    resp.headers_mut().insert(CONTENT_TYPE, HDR_TEXT.clone());
    resp
}

#[web::get("/baseline2")]
async fn baseline2(req: web::HttpRequest) -> web::HttpResponse {
    let sum = req
        .uri()
        .query()
        .map(parse_query_sum)
        .unwrap_or(0);
    let body = sum.to_string();
    let mut resp = web::HttpResponse::with_body(http::StatusCode::OK, body.into());
    resp.headers_mut().insert(SERVER, HDR_SERVER.clone());
    resp.headers_mut().insert(CONTENT_TYPE, HDR_TEXT.clone());
    resp
}

#[web::get("/json")]
async fn json(state: web::types::State<Arc<AppState>>) -> web::HttpResponse {
    let items: Vec<ProcessedItem> = state
        .dataset
        .iter()
        .map(|d| ProcessedItem {
            id: d.id,
            name: d.name.clone(),
            category: d.category.clone(),
            price: d.price,
            quantity: d.quantity,
            active: d.active,
            tags: d.tags.clone(),
            rating: RatingOut {
                score: d.rating.score,
                count: d.rating.count,
            },
            total: (d.price * d.quantity as f64 * 100.0).round() / 100.0,
        })
        .collect();
    let resp_data = JsonResponse {
        count: items.len(),
        items,
    };
    let content = serde_json::to_vec(&resp_data).unwrap_or_default();
    let mut resp =
        web::HttpResponse::with_body(http::StatusCode::OK, http::body::Body::Bytes(content.into()));
    resp.headers_mut().insert(SERVER, HDR_SERVER.clone());
    resp.headers_mut().insert(CONTENT_TYPE, HDR_JSON.clone());
    resp
}

fn config() -> SharedCfg {
    SharedCfg::new("httparena")
        .add(
            IoConfig::new()
                .set_read_buf(65535, 2048, 128)
                .set_write_buf(65535, 2048, 128),
        )
        .add(
            HttpServiceConfig::new()
                .set_keepalive(KeepAlive::Os)
                .set_client_timeout(Seconds::ZERO)
                .set_headers_read_rate(Seconds::ZERO, Seconds::ZERO, 0)
                .set_payload_read_rate(Seconds::ZERO, Seconds::ZERO, 0),
        )
        .into()
}

fn load_tls_config() -> Option<Arc<ServerConfig>> {
    let cert_path = std::env::var("TLS_CERT").unwrap_or_else(|_| "/certs/server.crt".to_string());
    let key_path = std::env::var("TLS_KEY").unwrap_or_else(|_| "/certs/server.key".to_string());
    let cert_file = std::fs::File::open(&cert_path).ok()?;
    let key_file = std::fs::File::open(&key_path).ok()?;
    let certs: Vec<_> = rustls_pemfile::certs(&mut io::BufReader::new(cert_file))
        .filter_map(|r| r.ok())
        .collect();
    let key = rustls_pemfile::private_key(&mut io::BufReader::new(key_file)).ok()??;
    let mut cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .ok()?;
    cfg.alpn_protocols = vec![b"h2".to_vec()];
    Some(Arc::new(cfg))
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let dataset = Arc::new(AppState {
        dataset: load_dataset(),
    });

    let tls_config = load_tls_config();

    let mut server = web::HttpServer::new({
        let ds = dataset.clone();
        async move || {
            web::App::new()
                .state(ds.clone())
                .service(pipeline)
                .service(baseline_get)
                .service(baseline_post)
                .service(baseline2)
                .service(json)
        }
    })
    .backlog(4096)
    .config(config())
    .bind("0.0.0.0:8080")?;

    if let Some(tls_cfg) = tls_config {
        server = server.bind_rustls("0.0.0.0:8443", &tls_cfg)?;
    }

    server.run().await
}
