use tokio::{select, spawn};
mod web;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let grpc_addr = "0.0.0.0:50051";
    let http_addr = "127.0.0.1:8080";

    // start HTTP-static server
    let http = spawn(async {
        web::run_static_server(http_addr)
            .await
            .expect("http server failed");
    });

    // start your existing tonic server
    let grpc = spawn(async {
        putty_grpc_server::run(grpc_addr)
            .await
            .expect("gRPC server failed");
    });

    // await both (Ctrl-C cancels both nicely via tokio::signal)
    select! {
        _ = http => {},
        _ = grpc => {},
    }
    Ok(())
}
