//! Very small HTTP server that
//! 1. serves everything in `../webui/dist` from disk during dev
//! 2. falls back to the same bundle embedded with `rust-embed`
//!
//! There is **no** proxying layer here – the React/TS frontend should talk
//! to the gRPC-Web endpoint on `http://<host>:50051` directly.  That
//! endpoint already has `CorsLayer::permissive()` on it.

use axum::{
    body::Body,
    http::{
        header::{HeaderValue, CONTENT_TYPE},
        Request, Response, StatusCode,
    },
    response::IntoResponse,
    routing::get_service,
    serve, Router,
};
use rust_embed::RustEmbed;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::{info, warn, Level};

/* ---------- embed the compiled bundle (for release builds) ------------ */

#[derive(RustEmbed)]
#[folder = "../webui/dist"]
struct Assets;

/* --------------------------- runner ----------------------------------- */

pub async fn run_static_server(addr: &str) -> anyhow::Result<()> {
    /* where we look for the bundle on disk */
    let www_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../webui/dist");
    info!("🔍 serving files from: {}", www_root.display());

    assert!(
        www_root.join("index.html").exists(),
        "frontend bundle not found – did you run  npm run build ?"
    );

    /* ── (1) Serve from disk whenever possible ───────────────────────── */
    let serve_dir = get_service(ServeDir::new(&www_root)).handle_error(|err| async move {
        warn!("ServeDir error: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    });

    /* ── (2) SPA fallback: embedded bundle (also catches 404) ─────────── */
    async fn spa_fallback(req: Request<Body>) -> impl IntoResponse {
        let path = req.uri().path().trim_start_matches('/');
        let asset = Assets::get(path).or_else(|| Assets::get("index.html")); // SPA
        match asset {
            Some(file) => {
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                let mut resp = Response::new(Body::from(file.data.into_owned()));
                resp.headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).unwrap());
                resp
            }
            None => (StatusCode::NOT_FOUND, "404").into_response(),
        }
    }

    /* ── compose the router ───────────────────────────────────────────── */
    let app = Router::new()
        .nest_service("/", serve_dir) // static files first
        .fallback(spa_fallback) // everything else → embedded SPA
        .layer(
            TraceLayer::new_for_http() // 1-line structured access log
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    /* ── run ──────────────────────────────────────────────────────────── */
    let listener = TcpListener::bind(addr).await?;
    info!("static files on http://{addr}");

    // Open the page once the listener is bound.
    let url_to_open = if addr.starts_with("0.0.0.0:") {
        let port = addr.split(':').nth(1).unwrap_or("8080");
        format!("http://127.0.0.1:{port}")
    } else {
        format!("http://{addr}")
    };

    // don’t block the server task
    tokio::spawn(async move {
        // it’s fine if this fails (headless boxes, etc.)
        let _ = webbrowser::open(&url_to_open);
    });

    serve(listener, app).await?;
    Ok(())
}
