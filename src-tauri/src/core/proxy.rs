use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::core::direct_claude_route::resolve_proxy_upstream_url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProxyConfig {
    pub target_os: String,
    pub target_arch: String,
    pub upstream_url: String,
    #[serde(default)]
    pub dynamic_upstream: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            target_os: String::from("Windows"),
            target_arch: String::from("x64"),
            upstream_url: String::from("https://anyrouter.top"),
            dynamic_upstream: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyStatus {
    pub running: bool,
    pub listen_port: u16,
    pub target_os: String,
    pub target_arch: String,
    pub upstream_url: String,
    pub dynamic_upstream: bool,
    pub error: Option<String>,
}

pub struct ProxyHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    config: Arc<RwLock<ProxyConfig>>,
    listen_port: u16,
}

impl ProxyHandle {
    pub fn new(config: ProxyConfig, port: u16) -> Self {
        Self {
            shutdown_tx: None,
            config: Arc::new(RwLock::new(config)),
            listen_port: port,
        }
    }

    pub async fn start(&mut self) -> Result<(), String> {
        if self.shutdown_tx.is_some() {
            return Ok(());
        }

        let addr: SocketAddr = format!("127.0.0.1:{}", self.listen_port)
            .parse()
            .map_err(|err| format!("invalid listen address: {err}"))?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|err| format!("无法绑定端口 {}: {err}", self.listen_port))?;

        let config = self.config.clone();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        tokio::spawn(async move {
            let graceful = async move {
                loop {
                    tokio::select! {
                        result = listener.accept() => {
                            match result {
                                Ok((stream, _peer)) => {
                                    let config = config.clone();
                                    tokio::spawn(async move {
                                        let io = TokioIo::new(stream);
                                        if let Err(err) = http1::Builder::new()
                                            .serve_connection(
                                                io,
                                                service_fn(move |req| {
                                                    handle_request(req, config.clone(), addr.port())
                                                }),
                                            )
                                            .await
                                        {
                                            crate::system::app_log::error("proxy.conn", err.to_string());
                                        }
                                    });
                                }
                                Err(err) => {
                                    crate::system::app_log::error("proxy.accept", err.to_string());
                                }
                            }
                        }
                        _ = &mut shutdown_rx => {
                            break;
                        }
                    }
                }
            };

            graceful.await;
        });

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }

    pub fn is_running(&self) -> bool {
        self.shutdown_tx.is_some()
    }

    pub fn status(&self) -> ProxyStatus {
        let config = self
            .config
            .read()
            .expect("proxy config lock should not be poisoned")
            .clone();
        ProxyStatus {
            running: self.is_running(),
            listen_port: self.listen_port,
            target_os: config.target_os,
            target_arch: config.target_arch,
            upstream_url: resolve_proxy_upstream_url(
                "",
                &config.upstream_url,
                config.dynamic_upstream,
                self.listen_port,
            )
            .upstream_url,
            dynamic_upstream: config.dynamic_upstream,
            error: None,
        }
    }

    pub fn update_config(&mut self, config: ProxyConfig) {
        *self
            .config
            .write()
            .expect("proxy config lock should not be poisoned") = config;
    }

    pub fn proxy_url(&self) -> Option<String> {
        if self.is_running() {
            Some(format!("http://127.0.0.1:{}", self.listen_port))
        } else {
            None
        }
    }
}

async fn handle_request(
    req: Request<Incoming>,
    config: Arc<RwLock<ProxyConfig>>,
    listen_port: u16,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let config = config
        .read()
        .expect("proxy config lock should not be poisoned")
        .clone();

    if req.method() == Method::OPTIONS {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::new()))
            .unwrap());
    }

    let upstream_path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let resolved_upstream = resolve_proxy_upstream_url(
        "",
        &config.upstream_url,
        config.dynamic_upstream,
        listen_port,
    );
    let upstream_url = format!(
        "{}{}",
        resolved_upstream.upstream_url.trim_end_matches('/'),
        upstream_path
    );

    let client = Client::builder()
        .build()
        .expect("reqwest client should build");

    let mut upstream_req = client
        .request(req.method().clone(), &upstream_url)
        .headers(reqwest::header::HeaderMap::new());

    for (name, value) in req.headers() {
        let header_name = name.as_str().to_ascii_lowercase();
        if header_name == "host"
            || header_name == "content-length"
            || header_name == "transfer-encoding"
        {
            continue;
        }
        if header_name == "x-stainless-os" {
            upstream_req = upstream_req.header("X-Stainless-OS", &config.target_os);
        } else if header_name == "x-stainless-arch" {
            upstream_req = upstream_req.header("X-Stainless-Arch", &config.target_arch);
        } else {
            upstream_req = upstream_req.header(name.as_str(), value.as_bytes());
        }
    }

    let content_type_is_json = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("application/json"))
        .unwrap_or(false);

    let mut body_bytes = match req.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => Bytes::new(),
    };

    if content_type_is_json && !body_bytes.is_empty() {
        if let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
            if let Some(obj) = json.as_object_mut() {
                if let Some(ctx_mgmt) = obj.get("context_management") {
                    if !ctx_mgmt.is_null() {
                        obj.insert("context_management".to_string(), serde_json::Value::Null);
                        body_bytes = Bytes::from(
                            serde_json::to_vec(&json).unwrap_or_else(|_| body_bytes.to_vec()),
                        );
                    }
                }
            }
        }
    }

    upstream_req = upstream_req.body(body_bytes);

    match upstream_req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let mut response = Response::builder().status(status);
            for (header_name, header_value) in resp.headers() {
                response = response.header(header_name.as_str(), header_value.as_bytes());
            }
            let resp_body = resp.bytes().await.unwrap_or_default();
            Ok(response.body(Full::new(resp_body)).unwrap())
        }
        Err(err) => Ok(Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Full::new(Bytes::from(format!(
                "proxy upstream error: {err}"
            ))))
            .unwrap()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::Incoming;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn proxy_rewrites_os_and_arch_headers() {
        let echo_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let (echo_shutdown_tx, echo_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let captured = Arc::new(Mutex::new(Vec::new()));

        let echo_listener = TcpListener::bind(echo_addr).await.unwrap();
        let echo_port = echo_listener.local_addr().unwrap().port();
        let echo_captured = captured.clone();
        tokio::spawn(async move {
            let mut echo_shutdown = echo_shutdown_rx;
            loop {
                tokio::select! {
                    result = echo_listener.accept() => {
                        let (stream, _) = result.unwrap();
                        let c = echo_captured.clone();
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            http1::Builder::new()
                                .serve_connection(io, service_fn(move |req: Request<Incoming>| {
                                    let c = c.clone();
                                    async move {
                                        {
                                            let mut h = c.lock().await;
                                            h.clear();
                                            for (name, value) in req.headers() {
                                                h.push((
                                                    name.to_string(),
                                                    value.to_str().unwrap_or("").to_string(),
                                                ));
                                            }
                                        }
                                        Ok::<_, Infallible>(
                                            Response::builder()
                                                .status(StatusCode::OK)
                                                .body(Full::new(Bytes::from("echo-ok")))
                                                .unwrap(),
                                        )
                                    }
                                }))
                                .await
                                .unwrap();
                        });
                    }
                    _ = &mut echo_shutdown => break,
                }
            }
        });

        let config = ProxyConfig {
            target_os: "Windows".to_string(),
            target_arch: "x64".to_string(),
            upstream_url: format!("http://127.0.0.1:{echo_port}"),
            dynamic_upstream: false,
        };

        let mut proxy = ProxyHandle::new(config, 0);
        proxy.listen_port = 0;
        let proxy_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let proxy_listener = TcpListener::bind(proxy_addr).await.unwrap();
        let proxy_port = proxy_listener.local_addr().unwrap().port();
        let (proxy_shutdown_tx, mut proxy_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        proxy.shutdown_tx = Some(proxy_shutdown_tx);
        let proxy_config = proxy.config.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = proxy_listener.accept() => {
                        let (stream, _) = result.unwrap();
                        let cfg = proxy_config.clone();
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            http1::Builder::new()
                                .serve_connection(io, service_fn(move |req| handle_request(req, cfg.clone(), proxy_port)))
                                .await
                                .unwrap();
                        });
                    }
                    _ = &mut proxy_shutdown_rx => break,
                }
            }
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://127.0.0.1:{proxy_port}/v1/messages"))
            .header("X-Stainless-OS", "MacOS")
            .header("X-Stainless-Arch", "arm64")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-token")
            .body(r#"{"model":"test"}"#)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);
        assert_eq!(resp.text().await.unwrap(), "echo-ok");

        let headers = captured.lock().await;
        let os_header = headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("x-stainless-os"));
        let arch_header = headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("x-stainless-arch"));
        let content_type = headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("content-type"));
        let auth = headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("authorization"));

        assert_eq!(
            os_header.map(|(_, v)| v.as_str()),
            Some("Windows"),
            "X-Stainless-OS should be rewritten to Windows"
        );
        assert_eq!(
            arch_header.map(|(_, v)| v.as_str()),
            Some("x64"),
            "X-Stainless-Arch should be rewritten to x64"
        );
        assert_eq!(
            content_type.map(|(_, v)| v.as_str()),
            Some("application/json"),
            "Content-Type should pass through"
        );
        assert_eq!(
            auth.map(|(_, v)| v.as_str()),
            Some("Bearer test-token"),
            "Authorization should pass through"
        );

        let _ = echo_shutdown_tx.send(());
    }

    #[tokio::test]
    async fn running_proxy_uses_updated_target_config() {
        let echo_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let (echo_shutdown_tx, echo_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let captured = Arc::new(Mutex::new(Vec::new()));

        let echo_listener = TcpListener::bind(echo_addr).await.unwrap();
        let echo_port = echo_listener.local_addr().unwrap().port();
        let echo_captured = captured.clone();
        tokio::spawn(async move {
            let mut echo_shutdown = echo_shutdown_rx;
            loop {
                tokio::select! {
                    result = echo_listener.accept() => {
                        let (stream, _) = result.unwrap();
                        let c = echo_captured.clone();
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            http1::Builder::new()
                                .serve_connection(io, service_fn(move |req: Request<Incoming>| {
                                    let c = c.clone();
                                    async move {
                                        {
                                            let mut h = c.lock().await;
                                            h.clear();
                                            for (name, value) in req.headers() {
                                                h.push((
                                                    name.to_string(),
                                                    value.to_str().unwrap_or("").to_string(),
                                                ));
                                            }
                                        }
                                        Ok::<_, Infallible>(
                                            Response::builder()
                                                .status(StatusCode::OK)
                                                .body(Full::new(Bytes::from("echo-ok")))
                                                .unwrap(),
                                        )
                                    }
                                }))
                                .await
                                .unwrap();
                        });
                    }
                    _ = &mut echo_shutdown => break,
                }
            }
        });

        let config = ProxyConfig {
            target_os: "MacOS".to_string(),
            target_arch: "arm64".to_string(),
            upstream_url: format!("http://127.0.0.1:{echo_port}"),
            dynamic_upstream: false,
        };

        let mut proxy = ProxyHandle::new(config, 0);
        let proxy_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let proxy_listener = TcpListener::bind(proxy_addr).await.unwrap();
        let proxy_port = proxy_listener.local_addr().unwrap().port();
        let (proxy_shutdown_tx, mut proxy_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        proxy.shutdown_tx = Some(proxy_shutdown_tx);
        let proxy_config = proxy.config.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = proxy_listener.accept() => {
                        let (stream, _) = result.unwrap();
                        let cfg = proxy_config.clone();
                        tokio::spawn(async move {
                            let io = TokioIo::new(stream);
                            http1::Builder::new()
                                .serve_connection(io, service_fn(move |req| handle_request(req, cfg.clone(), proxy_port)))
                                .await
                                .unwrap();
                        });
                    }
                    _ = &mut proxy_shutdown_rx => break,
                }
            }
        });

        proxy.update_config(ProxyConfig {
            target_os: "Windows".to_string(),
            target_arch: "x64".to_string(),
            upstream_url: format!("http://127.0.0.1:{echo_port}"),
            dynamic_upstream: false,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://127.0.0.1:{proxy_port}/v1/messages"))
            .header("X-Stainless-OS", "MacOS")
            .header("X-Stainless-Arch", "arm64")
            .body(r#"{"model":"test"}"#)
            .send()
            .await
            .unwrap();

        assert_eq!(resp.status().as_u16(), 200);

        let headers = captured.lock().await;
        let os_header = headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("x-stainless-os"));
        let arch_header = headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("x-stainless-arch"));

        assert_eq!(os_header.map(|(_, v)| v.as_str()), Some("Windows"));
        assert_eq!(arch_header.map(|(_, v)| v.as_str()), Some("x64"));

        let _ = echo_shutdown_tx.send(());
    }
}
