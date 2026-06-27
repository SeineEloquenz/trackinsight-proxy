//! trackinsight-proxy

use std::{env, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{anyhow, Context, Result};
use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use chaser_oxide::{Browser, ChaserPage};
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::Mutex;

#[derive(Clone)]
struct Config {
    origin: String,
    warm_url: String,
    warm_delay: Duration,
    chrome_host: String,
    chrome_port: u16,
}

impl Config {
    fn from_env() -> Self {
        let origin = env::var("TRACKINSIGHT_ORIGIN")
            .unwrap_or_else(|_| "https://www.trackinsight.com".to_string());
        let warm_url =
            env::var("WARM_URL").unwrap_or_else(|_| format!("{origin}/en/etf/US/QQQ/"));
        let warm_delay = Duration::from_secs(
            env::var("WARM_DELAY_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(6),
        );

        let chrome_url =
            env::var("CHROME_URL").unwrap_or_else(|_| "http://127.0.0.1:9222".to_string());
        let hostport = chrome_url
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/');
        let (chrome_host, chrome_port) = match hostport.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), p.parse().unwrap_or(9222)),
            None => (hostport.to_string(), 9222),
        };

        Self {
            origin,
            warm_url,
            warm_delay,
            chrome_host,
            chrome_port,
        }
    }
}

#[derive(Debug, Deserialize)]
struct FetchResult {
    status: u16,
    #[serde(default)]
    body: String,
}

/// Holds the live CDP connection. `_browser` must be kept alive; all access is
/// serialised by the outer `Mutex`.
struct Solver {
    cfg: Config,
    _browser: Browser,
    page: ChaserPage,
}

impl Solver {
    async fn connect(cfg: &Config) -> Result<Self> {
        // Resolve the Chrome host to an IP first: Chrome's DevTools endpoint
        // rejects (DNS-rebinding protection) any `Host` header that is not an IP
        // literal or `localhost`, so connecting by service name would be reset.
        let addr: SocketAddr =
            tokio::net::lookup_host((cfg.chrome_host.as_str(), cfg.chrome_port))
                .await
                .with_context(|| format!("resolving {}", cfg.chrome_host))?
                .next()
                .ok_or_else(|| anyhow!("no address for {}", cfg.chrome_host))?;

        let endpoint = format!("http://{addr}");
        let (browser, mut handler) = Browser::connect(endpoint).await?;
        tokio::spawn(async move { while handler.next().await.is_some() {} });

        let page = browser.new_page("about:blank").await?;
        // Best-effort script-injection stealth (UA, navigator.webdriver, …).
        if let Err(e) = page.enable_stealth_mode().await {
            tracing::warn!("enable_stealth_mode failed (continuing): {e}");
        }
        let chaser = ChaserPage::new(page);

        let solver = Self {
            cfg: cfg.clone(),
            _browser: browser,
            page: chaser,
        };
        solver.warm().await?;
        Ok(solver)
    }

    /// Navigate to the site so the AWS WAF SDK runs and mints the `aws-waf-token`,
    /// then confirm an API fetch actually succeeds.
    async fn warm(&self) -> Result<()> {
        let url_lit = serde_json::to_string(&self.cfg.warm_url)?;
        // The execution context is torn down by the navigation, so an error here
        // (e.g. "context destroyed") is expected and ignored.
        let _ = self
            .page
            .evaluate(format!("window.location.href = {url_lit}").as_str())
            .await;
        tokio::time::sleep(self.cfg.warm_delay).await;
        let probe = self.fetch_once("/data-api/funds/QQQ.json").await?;
        if probe.status == 200 {
            tracing::info!("warm: aws-waf-token established (probe 200)");
            Ok(())
        } else {
            Err(anyhow!("warm probe returned status {}", probe.status))
        }
    }

    /// Perform a single in-page `fetch` of `path` against the configured origin.
    async fn fetch_once(&self, path: &str) -> Result<FetchResult> {
        let url = format!("{}{}", self.cfg.origin, path);
        let url_lit = serde_json::to_string(&url)?;
        let script = format!(
            "(async () => {{ \
                try {{ \
                    const r = await fetch({url_lit}, {{ credentials: 'include' }}); \
                    return {{ status: r.status, body: await r.text() }}; \
                }} catch (e) {{ \
                    return {{ status: 0, body: String(e) }}; \
                }} \
            }})()"
        );
        let value: Option<Value> = self.page.evaluate(script.as_str()).await?;
        let value = value.ok_or_else(|| anyhow!("evaluate returned no value"))?;
        Ok(serde_json::from_value(value)?)
    }

    /// Proxy a request, re-warming + retrying once if the WAF challenges us
    /// (HTTP 202 / empty body) — e.g. after the ~5 minute token TTL.
    async fn proxy(&self, path: &str) -> Result<FetchResult> {
        let res = self.fetch_once(path).await?;
        if res.status == 202 || res.body.is_empty() {
            tracing::info!(path, "challenge/empty -> re-warming and retrying");
            self.warm().await.ok();
            return self.fetch_once(path).await;
        }
        Ok(res)
    }
}

type Shared = Arc<Mutex<Solver>>;

async fn connect_with_retry(cfg: &Config) -> Result<Solver> {
    let mut backoff = Duration::from_secs(1);
    for attempt in 1..=10u32 {
        match Solver::connect(cfg).await {
            Ok(solver) => return Ok(solver),
            Err(e) => {
                tracing::warn!(attempt, "connect failed: {e}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(15));
            }
        }
    }
    Solver::connect(cfg).await.context("final connect attempt")
}

async fn health() -> StatusCode {
    StatusCode::OK
}

async fn proxy_handler(State(shared): State<Shared>, uri: Uri) -> Response {
    let path = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/")
        .to_string();

    if !(path.starts_with("/data-api/") || path.starts_with("/search-api/")) {
        return (StatusCode::NOT_FOUND, "only /data-api/* and /search-api/* are proxied")
            .into_response();
    }

    // First attempt.
    let first = shared.lock().await.proxy(&path).await;
    let res = match first {
        Ok(res) => Ok(res),
        Err(e) => {
            // The CDP connection likely dropped (sidecar restart). Reconnect once.
            tracing::warn!(path, error = %e, "proxy failed; reconnecting");
            let cfg = shared.lock().await.cfg.clone();
            match connect_with_retry(&cfg).await {
                Ok(solver) => {
                    *shared.lock().await = solver;
                    shared.lock().await.proxy(&path).await
                }
                Err(e) => Err(e),
            }
        }
    };

    match res {
        Ok(res) if res.status >= 200 => Response::builder()
            .status(StatusCode::from_u16(res.status).unwrap_or(StatusCode::BAD_GATEWAY))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(res.body))
            .unwrap(),
        Ok(res) => {
            tracing::warn!(path, status = res.status, "upstream fetch failed");
            (StatusCode::BAD_GATEWAY, res.body).into_response()
        }
        Err(e) => {
            tracing::error!(path, error = %e, "proxy error");
            (StatusCode::BAD_GATEWAY, format!("proxy error: {e}")).into_response()
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = Config::from_env();
    let port: u16 = env::var("SOLVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8191);

    tracing::info!(
        chrome = %format!("{}:{}", cfg.chrome_host, cfg.chrome_port),
        origin = %cfg.origin,
        "connecting to Chrome and warming"
    );
    let solver = connect_with_retry(&cfg).await.context("initial connect")?;
    let shared: Shared = Arc::new(Mutex::new(solver));

    // Periodically re-warm so the token never goes stale under low traffic.
    {
        let shared = shared.clone();
        let cfg = cfg.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(180)).await;
                let warmed = shared.lock().await.warm().await;
                if let Err(e) = warmed {
                    tracing::warn!("re-warm failed ({e}); reconnecting");
                    if let Ok(solver) = connect_with_retry(&cfg).await {
                        *shared.lock().await = solver;
                    }
                }
            }
        });
    }

    let app = Router::new()
        .route("/healthz", get(health))
        .fallback(proxy_handler)
        .with_state(shared);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
