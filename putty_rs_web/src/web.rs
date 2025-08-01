use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::get_service,
    Router,
    serve,
};
                      // ← use hyper directly
use rust_embed::RustEmbed;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{Level, info};

/* ───────────── bundled assets ───────────── */
#[derive(RustEmbed)]
#[folder = "../webui/dist"]
struct Assets;

/* ───────────── server runner ───────────── */
pub async fn run_static_server(addr: &str) -> anyhow::Result<()> {
    /* where to look on disk (handy during dev) */
    let www_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../webui/dist");
    info!("🔍 serving files from: {}", www_root.display());
    assert!(
        www_root.join("index.html").exists(),
        "frontend bundle not found – did you run  npm run build  ?"
    );

    /* 1 ─ try disk first */
    let serve_dir = get_service(ServeDir::new(&www_root))
        .handle_error(|err| async move {
            tracing::warn!("ServeDir error: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        });

    /* 2 ─ fall back to embedded content */
    async fn fallback(req: Request<Body>) -> impl IntoResponse {
        let path = req.uri().path().trim_start_matches('/');
        let asset = Assets::get(path).or_else(|| Assets::get("index.html")); // SPA
        match asset {
            Some(d) => Response::builder()
                .header(
                    "Content-Type",
                    mime_guess::from_path(path)
                        .first_or_octet_stream()
                        .as_ref(),
                )
                .body(Body::from(d.data.into_owned()))
                .unwrap(),
            None => (StatusCode::NOT_FOUND, "404").into_response(),
        }
    }

    let app = Router::new()
        .fallback(fallback)                  // any request not handled below
        .nest_service("/", serve_dir)        // static files
        .layer(                              // one-line access log
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    /* run */
    let listener = TcpListener::bind(addr).await?;
    info!("static files on http://{addr}");
    serve(listener, app).await?;

    Ok(())
}
