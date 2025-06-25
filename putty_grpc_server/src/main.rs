use std::net::SocketAddr;

use putty_core::{connections::connection::Connection, ConnectionManager};
use tokio::sync::mpsc;
use tonic::{
    transport::Server as TonicServer, // gRPC transport server
    Request as TReq,
    Response as TResp,
    Status,
};
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;
use tracing::info;

// ── generated protobuf code ───────────────────────────────────────────────────
pub mod putty_interface {
    tonic::include_proto!("putty_interface");
}
use putty_interface::remote_connection_server::{RemoteConnection, RemoteConnectionServer};
use putty_interface::*;

// ── gRPC service backed by putty_core ─────────────────────────────────────────
#[derive(Clone)]
struct ConnectionService {
    manager: ConnectionManager,
}

impl ConnectionService {
    fn new() -> Self {
        Self {
            manager: ConnectionManager::new(),
        }
    }
}

#[tonic::async_trait]
impl RemoteConnection for ConnectionService {
    type ReadStream = tokio_stream::wrappers::ReceiverStream<Result<ByteChunk, Status>>;

    async fn create_remote_connection(
        &self,
        req: TReq<CreateRequest>,
    ) -> Result<TResp<ConnectionId>, Status> {
        let id = uuid::Uuid::new_v4().to_string();
        let conn: Box<dyn Connection + Send + Unpin + 'static> = match req
            .into_inner()
            .kind
            .ok_or(Status::invalid_argument("kind"))?
        {
            create_request::Kind::Serial(s) => Box::new(
                putty_core::connections::serial::SerialConnection::new(s.port, s.baud),
            ),
            create_request::Kind::Ssh(s) => {
                Box::new(putty_core::connections::ssh::SshConnection::new(
                    s.host,
                    s.port as u16,
                    s.user,
                    s.password,
                ))
            }
        };

        self.manager
            .add_connection(id.clone(), conn)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(TResp::new(ConnectionId { id }))
    }

    async fn write(&self, req: TReq<WriteRequest>) -> Result<TResp<Empty>, Status> {
        let m = req.into_inner();
        self.manager
            .write_bytes(&m.id, &m.data)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(TResp::new(Empty {}))
    }

    async fn stop(&self, req: TReq<ConnectionId>) -> Result<TResp<Empty>, Status> {
        self.manager
            .stop_connection(&req.into_inner().id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(TResp::new(Empty {}))
    }

    async fn read(&self, req: TReq<ConnectionId>) -> Result<TResp<Self::ReadStream>, Status> {
        let id = req.into_inner().id;
        let mut rx = self
            .manager
            .subscribe(&id)
            .await
            .ok_or(Status::not_found("no such connection"))?;

        let (tx, rx_stream) = mpsc::channel::<Result<ByteChunk, Status>>(64);
        // forward every chunk from ConnectionManager → gRPC stream
        tokio::spawn(async move {
            while let Ok(chunk) = rx.recv().await {
                if tx.send(Ok(ByteChunk { data: chunk })).await.is_err() {
                    break; // client hung up
                }
            }
        });

        Ok(TResp::new(tokio_stream::wrappers::ReceiverStream::new(
            rx_stream,
        )))
    }
}

// ── main ──────────────────────────────────────────────────────────────────────
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let server = RemoteConnectionServer::new(ConnectionService::new());

    let addr: SocketAddr = ([127, 0, 0, 1], 50051).into();
    info!("gRPC-Web listening on http://{addr}");

    TonicServer::builder()
        .accept_http1(true) // gRPC-Web needs h1
        .layer(GrpcWebLayer::new()) // translate to gRPC-Web
        .layer(CorsLayer::permissive()) // allow browser calls
        .add_service(server)
        .serve(addr)
        .await?;

    Ok(())
}
