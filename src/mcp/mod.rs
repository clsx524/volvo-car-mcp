mod server;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

use server::VolvoMcp;

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub host: String,
    pub port: u16,
    pub bearer_token: String,
    pub store_path: PathBuf,
    pub profile_name: String,
    pub base_url: String,
    pub allowed_hosts: Option<Vec<String>>,
}

pub async fn run(config: McpServerConfig) -> Result<()> {
    if config.bearer_token.trim().is_empty() {
        return Err(anyhow!(
            "a non-empty bearer token is required; set VC_MCP_BEARER_TOKEN or pass --bearer-token"
        ));
    }
    let bearer_token = Arc::new(config.bearer_token.trim().to_owned());

    let mut http_config = StreamableHttpServerConfig::default().with_json_response(true);
    match config.allowed_hosts.as_deref() {
        Some([first, rest @ ..]) => {
            let mut hosts = vec![first.to_owned()];
            hosts.extend(rest.iter().map(|host| host.to_owned()));
            http_config = http_config.with_allowed_hosts(hosts);
        }
        _ => {
            eprintln!(
                "warning: no allowed hosts configured (VC_MCP_ALLOWED_HOSTS); Host header validation is disabled and bearer auth is the only request gate"
            );
            http_config = http_config.disable_allowed_hosts();
        }
    }

    let cancellation_token = tokio_util::sync::CancellationToken::new();
    http_config = http_config.with_cancellation_token(cancellation_token.child_token());

    let store_path = config.store_path.clone();
    let profile_name = config.profile_name.clone();
    let base_url = config.base_url.clone();

    let service = StreamableHttpService::new(
        move || {
            Ok(VolvoMcp::new(
                store_path.clone(),
                profile_name.clone(),
                base_url.clone(),
            ))
        },
        LocalSessionManager::default().into(),
        http_config,
    );

    let protected = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(bearer_token, require_bearer));
    let router = Router::new()
        .route("/healthz", axum::routing::get(health))
        .merge(protected);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind MCP server on {addr}"))?;
    eprintln!("volvo-car-mcp: serving MCP on http://{addr}/mcp (bearer auth enabled)");
    eprintln!("volvo-car-mcp: health probe at http://{addr}/healthz (unauthenticated)");

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            cancellation_token.cancel();
        })
        .await
        .context("MCP server terminated unexpectedly")?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn require_bearer(
    State(expected): State<Arc<String>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let authorized = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| token_matches(value, &expected));
    if !authorized {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    next.run(request).await
}

fn token_matches(header_value: &str, expected: &str) -> bool {
    let mut parts = header_value.splitn(2, ' ');
    let scheme = parts.next().unwrap_or_default();
    let token = parts.next().unwrap_or_default().trim();
    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() {
        return false;
    }
    constant_time_eq(token.as_bytes(), expected.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::{constant_time_eq, token_matches};

    #[test]
    fn accepts_matching_bearer_token() {
        assert!(token_matches("Bearer secret-token", "secret-token"));
        assert!(token_matches("bearer secret-token", "secret-token"));
        assert!(token_matches("BEARER  token  ", "token"));
    }

    #[test]
    fn rejects_mismatched_or_malformed_headers() {
        assert!(!token_matches("Bearer wrong-token", "secret-token"));
        assert!(!token_matches("secret-token", "secret-token"));
        assert!(!token_matches("Basic secret-token", "secret-token"));
        assert!(!token_matches("Bearer", "secret-token"));
        assert!(!token_matches("Bearer ", "secret-token"));
        assert!(!token_matches("", "secret-token"));
    }

    #[test]
    fn constant_time_eq_compares_bytes() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"", b""));
    }
}
