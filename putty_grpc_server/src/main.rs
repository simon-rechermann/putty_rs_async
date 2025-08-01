

// ── main ──────────────────────────────────────────────────────────────────────
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    putty_grpc_server::run("0.0.0.0:50051").await
}
