use axum::{
    body::Body,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::get_service,
    Router,
};
use rust_embed::RustEmbed;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "../webui/dist"]          // <── compiled by build.rs
struct Assets;

pub async fn run_static_server(addr: &str) -> anyhow::Result<()> {
    let www_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../webui/dist");          // or whatever you use

    println!("🔍 serving files from: {}", www_root.display());
    assert!(www_root.exists(), "frontend bundle not found");
    // 1 · try to serve from disk (helpful during dev)
    let serve_dir = get_service(ServeDir::new("../webui/dist"))
        .handle_error(|_| async { StatusCode::INTERNAL_SERVER_ERROR });

    // 2 · fall back to embedded assets when the file is missing
    async fn fallback(req: Request<Body>) -> impl IntoResponse {
        let path = req.uri().path().trim_start_matches('/');
        let data = Assets::get(path)
            .or_else(|| Assets::get("index.html"));     // SPA fallback
        match data {
            Some(d) => Response::builder()
                .header("Content-Type", mime_guess::from_path(path).first_or_octet_stream().as_ref())
                .body(Body::from(d.data.into_owned()))
                .unwrap(),
            None => (StatusCode::NOT_FOUND, "404").into_response(),
        }
    }

    let app = Router::new()
        .fallback(fallback)
        .nest_service("/", serve_dir);

    axum::serve(TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
