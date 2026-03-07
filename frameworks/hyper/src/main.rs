use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::{io, thread};

use http::header::{CONTENT_TYPE, SERVER};
use http::{HeaderValue, Request, Response, StatusCode};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::{http1, http2};
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioExecutor};
use rustls::ServerConfig;
use serde::{Deserialize, Serialize};
use socket2::{Domain, SockAddr, Socket};
use tokio::net::TcpListener;
use tokio::runtime;
use tokio_rustls::TlsAcceptor;

static SERVER_HEADER: HeaderValue = HeaderValue::from_static("hyper");
static APPLICATION_JSON: HeaderValue = HeaderValue::from_static("application/json");
static TEXT_PLAIN: HeaderValue = HeaderValue::from_static("text/plain");
static OK_BODY: &[u8] = b"ok";

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

fn load_dataset() -> Vec<DatasetItem> {
    let path = std::env::var("DATASET_PATH").unwrap_or_else(|_| "/data/dataset.json".to_string());
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn parse_query_params(query: Option<&str>) -> i64 {
    let mut sum: i64 = 0;
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some(val) = pair.split('=').nth(1) {
                if let Ok(n) = val.parse::<i64>() {
                    sum += n;
                }
            }
        }
    }
    sum
}

fn pipeline_response() -> Result<Response<BoxBody<Bytes, Infallible>>, http::Error> {
    Response::builder()
        .header(SERVER, SERVER_HEADER.clone())
        .header(CONTENT_TYPE, TEXT_PLAIN.clone())
        .body(Full::from(OK_BODY).boxed())
}

fn baseline_get(query: Option<&str>) -> Result<Response<BoxBody<Bytes, Infallible>>, http::Error> {
    let sum = parse_query_params(query);
    let body = sum.to_string();
    Response::builder()
        .header(SERVER, SERVER_HEADER.clone())
        .header(CONTENT_TYPE, TEXT_PLAIN.clone())
        .body(Full::from(body).boxed())
}

async fn baseline_post(
    query: Option<&str>,
    req: Request<Incoming>,
) -> Result<Response<BoxBody<Bytes, Infallible>>, http::Error> {
    let mut sum = parse_query_params(query);
    let body_bytes = req.collect().await.map(|b| b.to_bytes()).unwrap_or_default();
    if let Ok(s) = std::str::from_utf8(&body_bytes) {
        if let Ok(n) = s.trim().parse::<i64>() {
            sum += n;
        }
    }
    let body = sum.to_string();
    Response::builder()
        .header(SERVER, SERVER_HEADER.clone())
        .header(CONTENT_TYPE, TEXT_PLAIN.clone())
        .body(Full::from(body).boxed())
}

fn json_response(dataset: &[DatasetItem]) -> Result<Response<BoxBody<Bytes, Infallible>>, http::Error> {
    let items: Vec<ProcessedItem> = dataset
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
    let resp = JsonResponse {
        count: items.len(),
        items,
    };
    let content = serde_json::to_vec(&resp).unwrap_or_default();
    Response::builder()
        .header(SERVER, SERVER_HEADER.clone())
        .header(CONTENT_TYPE, APPLICATION_JSON.clone())
        .body(Full::from(content).boxed())
}

fn not_found() -> Result<Response<BoxBody<Bytes, Infallible>>, http::Error> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Empty::new().boxed())
}

fn create_socket(addr: SocketAddr) -> io::Result<Socket> {
    let domain = Domain::IPV4;
    let socket = Socket::new(domain, socket2::Type::STREAM, None)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_reuse_address(true)?;
    socket.set_nodelay(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&SockAddr::from(addr))?;
    socket.listen(4096)?;
    Ok(socket)
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
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .ok()?;
    config.alpn_protocols = vec![b"h2".to_vec()];
    Some(Arc::new(config))
}

fn main() -> io::Result<()> {
    let threads = num_cpus::get();
    let dataset = Arc::new(load_dataset());
    let tls_config = load_tls_config();

    for _ in 1..threads {
        let ds = dataset.clone();
        let tls = tls_config.clone();
        thread::spawn(move || {
            let rt = runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, serve(ds, tls)).unwrap();
        });
    }

    let rt = runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, serve(dataset, tls_config))
}

fn make_service(ds: Arc<Vec<DatasetItem>>) -> impl Fn(Request<Incoming>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response<BoxBody<Bytes, Infallible>>, http::Error>> + Send>> + Clone {
    move |req: Request<Incoming>| {
        let ds = ds.clone();
        Box::pin(async move {
            let path = req.uri().path();
            let query = req.uri().query().map(|q| q.to_string());
            match path {
                "/pipeline" => pipeline_response(),
                "/baseline11" => {
                    if req.method() == http::Method::POST {
                        baseline_post(query.as_deref(), req).await
                    } else {
                        baseline_get(query.as_deref())
                    }
                }
                "/baseline2" => baseline_get(query.as_deref()),
                "/json" => json_response(&ds),
                _ => not_found(),
            }
        })
    }
}

async fn serve(dataset: Arc<Vec<DatasetItem>>, tls_config: Option<Arc<ServerConfig>>) -> io::Result<()> {
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8080));
    let socket = create_socket(addr)?;
    let listener = TcpListener::from_std(socket.into())?;

    let mut http = http1::Builder::new();
    http.pipeline_flush(true);

    // Spawn H2 TLS listener if certs available
    if let Some(tls_cfg) = tls_config {
        let acceptor = TlsAcceptor::from(tls_cfg);
        let h2_addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8443));
        let h2_socket = create_socket(h2_addr)?;
        let h2_listener = TcpListener::from_std(h2_socket.into())?;
        let ds = dataset.clone();
        tokio::task::spawn_local(async move {
            loop {
                let (stream, _) = match h2_listener.accept().await {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let acceptor = acceptor.clone();
                let ds = ds.clone();
                tokio::task::spawn_local(async move {
                    let tls_stream = match acceptor.accept(stream).await {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    let io = TokioIo::new(tls_stream);
                    let svc = make_service(ds);
                    let _ = http2::Builder::new(TokioExecutor::new())
                        .serve_connection(io, service_fn(svc))
                        .await;
                });
            }
        });
    }

    loop {
        let (stream, _) = listener.accept().await?;
        let http = http.clone();
        let ds = dataset.clone();
        tokio::task::spawn_local(async move {
            let io = TokioIo::new(stream);
            let svc = make_service(ds);
            let _ = http.serve_connection(io, service_fn(svc)).await;
        });
    }
}
