//! trackinsight-proxy

use std::{env, net::SocketAddr, sync::Arc, time::Duration};

const WARM_INTERVAL: Duration = Duration::from_secs(180);
const RECONNECT_BACKOFF_MIN: Duration = Duration::from_secs(1);
const RECONNECT_BACKOFF_MAX: Duration = Duration::from_secs(60);

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
        let warm_url = env::var("WARM_URL").unwrap_or_else(|_| format!("{origin}/en/etf/US/QQQ/"));
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

/// Live CDP connection to Chrome; all access is serialised by the outer `Mutex`.
struct Solver {
    cfg: Config,
    browser: Browser,
    page: ChaserPage,
}

impl Solver {
    async fn connect(cfg: &Config) -> Result<Self> {
        // Chrome's DevTools rejects a `Host` that isn't an IP or `localhost`
        // (DNS-rebinding guard), so connect by resolved IP, not the hostname.
        let addr: SocketAddr = tokio::net::lookup_host((cfg.chrome_host.as_str(), cfg.chrome_port))
            .await
            .with_context(|| format!("resolving {}", cfg.chrome_host))?
            .next()
            .ok_or_else(|| anyhow!("no address for {}", cfg.chrome_host))?;

        let endpoint = format!("http://{addr}");
        let (browser, mut handler) = Browser::connect(endpoint).await?;
        tokio::spawn(async move { while handler.next().await.is_some() {} });

        let page = browser.new_page("about:blank").await?;
        if let Err(e) = page.enable_stealth_mode().await {
            tracing::warn!("enable_stealth_mode failed (continuing): {e}");
        }
        let chaser = ChaserPage::new(page);

        let solver = Self {
            cfg: cfg.clone(),
            browser,
            page: chaser,
        };
        solver.sweep().await;
        if let Err(e) = solver.warm().await {
            solver.close().await;
            return Err(e);
        }
        Ok(solver)
    }

    /// `Page` has no `Drop` impl, and the `Browser` we attach to owns no child
    /// process, so a tab not closed here stays open for the browser's lifetime.
    async fn close(&self) {
        let page = self.page.raw_page().clone();
        if tokio::time::timeout(Duration::from_secs(10), page.close())
            .await
            .is_err()
        {
            tracing::warn!("closing tab timed out");
        }
    }

    /// Close tabs stranded by earlier connections. Assumes this proxy is the only
    /// client of the browser.
    async fn sweep(&self) {
        let keep = self.page.raw_page().target_id().clone();
        let pages = match self.browser.pages().await {
            Ok(pages) => pages,
            Err(e) => {
                tracing::warn!("sweep: listing pages failed: {e}");
                return;
            }
        };

        let mut closed = 0usize;
        for page in pages {
            if *page.target_id() == keep {
                continue;
            }
            match tokio::time::timeout(Duration::from_secs(10), page.close()).await {
                Ok(Ok(())) => closed += 1,
                Ok(Err(e)) => tracing::warn!("sweep: close failed: {e}"),
                Err(_) => tracing::warn!("sweep: close timed out"),
            }
        }
        if closed > 0 {
            tracing::info!(closed, "sweep: closed stranded tabs");
        }
    }

    /// Navigate the page so the AWS WAF SDK mints the `aws-waf-token`, then
    /// confirm an API fetch succeeds.
    async fn warm(&self) -> Result<()> {
        let url_lit = serde_json::to_string(&self.cfg.warm_url)?;
        // Navigation tears down the execution context, so ignore the error.
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

    /// Open a fresh tab on the same connection and re-warm it. Errors if the
    /// connection is gone, so the caller can fall back to a full reconnect.
    async fn relaunch(&mut self) -> Result<()> {
        // Close the old tab so they don't accumulate.
        let _ = self.page.raw_page().clone().close().await;

        let page = self.browser.new_page("about:blank").await?;
        if let Err(e) = page.enable_stealth_mode().await {
            tracing::warn!("enable_stealth_mode failed (continuing): {e}");
        }
        self.page = ChaserPage::new(page);
        self.warm().await
    }

    /// Proxy a request; re-warm and retry once if the result looks degraded.
    async fn proxy(&self, path: &str) -> Result<FetchResult> {
        let res = self.fetch_once(path).await?;
        if Self::is_degraded(&res) {
            tracing::info!(
                path,
                status = res.status,
                "degraded -> re-warming and retrying"
            );
            self.warm().await.ok();
            return self.fetch_once(path).await;
        }
        Ok(res)
    }

    /// Degraded = WAF challenge (202), empty body, or in-page fetch threw (0).
    fn is_degraded(res: &FetchResult) -> bool {
        res.status == 0 || res.status == 202 || res.body.is_empty()
    }
}

/// `None` while there is no live Chrome session; `maintain` fills it in.
type Shared = Arc<Mutex<Option<Solver>>>;

#[derive(Clone)]
struct AppState {
    solver: Shared,
    cfg: Config,
}

/// Own the connection for the process lifetime
async fn maintain(shared: Shared, cfg: Config) {
    let mut backoff = RECONNECT_BACKOFF_MIN;

    loop {
        // Bind before the branch: holding the guard into the block would deadlock
        // against the store below.
        let idle = shared.lock().await.is_none();

        if idle {
            match Solver::connect(&cfg).await {
                Ok(solver) => {
                    tracing::info!("connected to Chrome");
                    *shared.lock().await = Some(solver);
                    backoff = RECONNECT_BACKOFF_MIN;
                }
                Err(e) => {
                    tracing::warn!(retry_in = ?backoff, "connect failed: {e}");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(RECONNECT_BACKOFF_MAX);
                }
            }
            continue;
        }

        tokio::time::sleep(WARM_INTERVAL).await;

        let warmed = {
            let guard = shared.lock().await;
            match guard.as_ref() {
                Some(solver) => solver.warm().await,
                None => continue,
            }
        };
        if let Err(e) = warmed {
            tracing::warn!("re-warm failed ({e}); recovering");
            recover(&shared, &cfg, true).await;
        }
    }
}

/// Recover a degraded session
async fn recover(shared: &Shared, cfg: &Config, try_relaunch: bool) {
    if try_relaunch {
        // `new_page`/`close` can hang on a half-dead connection; bound it.
        let relaunched = {
            let mut guard = shared.lock().await;
            match guard.as_mut() {
                Some(solver) => {
                    tokio::time::timeout(Duration::from_secs(45), solver.relaunch()).await
                }
                None => Ok(Err(anyhow!("no live session"))),
            }
        };

        match relaunched {
            Ok(Ok(())) => return,
            Ok(Err(e)) => tracing::warn!("relaunch failed ({e}); reconnecting to Chrome"),
            Err(_) => tracing::warn!("relaunch timed out; reconnecting to Chrome"),
        }
    }

    let old = shared.lock().await.take();
    if let Some(old) = old {
        old.close().await;
    }

    match Solver::connect(cfg).await {
        Ok(solver) => *shared.lock().await = Some(solver),
        Err(e) => tracing::warn!("reconnect failed ({e}); retrying in the background"),
    }
}

async fn health() -> StatusCode {
    StatusCode::OK
}

/// Run `proxy` against the live session, or `None` if there isn't one.
async fn try_proxy(shared: &Shared, path: &str) -> Option<Result<FetchResult>> {
    let guard = shared.lock().await;
    match guard.as_ref() {
        Some(solver) => Some(solver.proxy(path).await),
        None => None,
    }
}

fn unavailable() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "no live Chrome session; retrying in the background",
    )
        .into_response()
}

async fn proxy_handler(State(state): State<AppState>, uri: Uri) -> Response {
    let path = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/")
        .to_string();

    if !(path.starts_with("/data-api/") || path.starts_with("/search-api/")) {
        return (
            StatusCode::NOT_FOUND,
            "only /data-api/* and /search-api/* are proxied",
        )
            .into_response();
    }

    let shared = &state.solver;

    let Some(first) = try_proxy(shared, &path).await else {
        return unavailable();
    };

    // `Err` = connection dropped (reconnect); `Ok` degraded = page bad but
    // connection alive (relaunch first).
    let (degraded, try_relaunch) = match &first {
        Err(_) => (true, false),
        Ok(res) => (Solver::is_degraded(res), true),
    };

    let res = if degraded {
        tracing::warn!(path, "session degraded; recovering");
        recover(shared, &state.cfg, try_relaunch).await;
        match try_proxy(shared, &path).await {
            Some(res) => res,
            None => return unavailable(),
        }
    } else {
        first
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
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
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
    let shared: Shared = Arc::new(Mutex::new(None));
    tokio::spawn(maintain(shared.clone(), cfg.clone()));

    let app = Router::new()
        .route("/healthz", get(health))
        .fallback(proxy_handler)
        .with_state(AppState {
            solver: shared,
            cfg,
        });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
