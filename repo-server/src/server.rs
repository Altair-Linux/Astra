use axum::Router;
use std::net::SocketAddr;
use std::path::Path;
use tower_http::services::ServeDir;

/// serves a repository directory over http.
///
/// the directory should contain:
/// - `index.json` — package index
/// - `packages/` — package files
/// - `signatures/` — signature files
pub async fn serve_repository(repo_dir: &Path, bind: SocketAddr) -> anyhow::Result<()> {
    let repo_dir = repo_dir.canonicalize()?;

    tracing::info!("Serving repository from {:?} on {}", repo_dir, bind);

    let app = Router::new().nest_service("/", ServeDir::new(&repo_dir));

    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!("Repository server listening on http://{}", bind);

    axum::serve(listener, app).await?;

    Ok(())
}
