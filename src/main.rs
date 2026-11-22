//! Serves TrackInsight's fund API for Ghostfolio from the data fund issuers publish.

use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde_json::{json, Value};

mod breakdown;
mod cache;
mod isin;
mod issuers;
mod normalize;
mod sheet;

use cache::Memo;
use issuers::Issuers;

struct App {
    issuers: Issuers,
    documents: Memo<String, Option<Arc<Value>>>,
}

type Shared = Arc<App>;

impl App {
    async fn holdings(&self, isin: &str) -> Result<Option<Arc<Value>>> {
        self.documents
            .get(&isin.to_owned(), || async {
                let found = self.issuers.breakdown(isin).await?;
                Ok(found.map(|(issuer, breakdown)| {
                    let document = breakdown.render();
                    tracing::info!(isin, issuer, weight = %document["weight"], "loaded holdings");
                    Arc::new(document)
                }))
            })
            .await
    }
}

fn upstream_error(e: anyhow::Error) -> StatusCode {
    tracing::warn!("{e:#}");
    StatusCode::BAD_GATEWAY
}

async fn search(State(app): State<Shared>, Path((query, _)): Path<(String, String)>) -> Response {
    let empty = Json(json!({ "results": { "count": 0, "docs": [] } }));
    let Some(isin) = isin::find(&query) else {
        return empty.into_response();
    };
    match app.holdings(&isin).await {
        Ok(Some(_)) => {
            Json(json!({ "results": { "count": 1, "docs": [{ "ticker": isin }] } })).into_response()
        }
        Ok(None) => empty.into_response(),
        Err(e) => upstream_error(e).into_response(),
    }
}

/// Resolves the `{ISIN}.json` file name of the data API to a supported fund's holdings.
async fn document(app: &App, file: &str) -> Result<(String, Arc<Value>), StatusCode> {
    let isin = file
        .strip_suffix(".json")
        .filter(|s| isin::is_valid(s))
        .ok_or(StatusCode::NOT_FOUND)?;
    match app.holdings(isin).await {
        Ok(Some(doc)) => Ok((isin.to_owned(), doc)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => Err(upstream_error(e)),
    }
}

async fn funds(State(app): State<Shared>, Path(file): Path<String>) -> Response {
    match document(&app, &file).await {
        Ok((isin, _)) => Json(json!({ "isins": [isin] })).into_response(),
        Err(status) => status.into_response(),
    }
}

async fn holdings(State(app): State<Shared>, Path(file): Path<String>) -> Response {
    match document(&app, &file).await {
        Ok((_, doc)) => Json(doc.as_ref().clone()).into_response(),
        Err(status) => status.into_response(),
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
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("a TLS crypto provider is already installed"))?;

    let host: IpAddr = match env::var("LISTEN_HOST") {
        Ok(s) => s
            .parse()
            .with_context(|| format!("invalid LISTEN_HOST {s:?}"))?,
        Err(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    let port: u16 = match env::var("LISTEN_PORT") {
        Ok(s) => s
            .parse()
            .with_context(|| format!("invalid LISTEN_PORT {s:?}"))?,
        Err(_) => 8191,
    };

    let app = Arc::new(App {
        issuers: Issuers::new()?,
        documents: Memo::default(),
    });
    let router = Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route("/search-api/search_v2/{query}/{*rest}", get(search))
        .route("/data-api/funds/{file}", get(funds))
        .route("/data-api/holdings/{file}", get(holdings))
        .with_state(app);

    let addr = SocketAddr::new(host, port);
    tracing::info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}
