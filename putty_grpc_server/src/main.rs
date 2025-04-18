pub mod putty {
    tonic::include_proto!("putty"); // generated by build.rs
}

use putty::terminal_server::Terminal;
use putty::*;

use putty_core::ConnectionManager;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
struct TerminalSvc {
    mgr: ConnectionManager,
}

impl TerminalSvc {
    fn new(mgr: ConnectionManager) -> Self {
        Self { mgr }
    }
}

#[tonic::async_trait]
impl Terminal for TerminalSvc {
    async fn create_connection(
        &self,
        req: Request<CreateRequest>,
    ) -> Result<Response<ConnectionId>, Status> {
        let id = uuid::Uuid::new_v4().to_string();
        let creation = req.into_inner();
        let handle = match creation
            .kind
            .ok_or(Status::invalid_argument("missing kind"))?
        {
            create_request::Kind::Serial(s) => {
                let conn = putty_core::connections::serial::SerialConnection::new(s.port, s.baud);
                self.mgr.add_connection(id.clone(), Box::new(conn)).await
            }
            create_request::Kind::Ssh(s) => {
                let conn = putty_core::connections::ssh::SshConnection::new(
                    s.host,
                    s.port as u16,
                    s.user,
                    s.pass,
                );
                self.mgr.add_connection(id.clone(), Box::new(conn)).await
            }
        }
        .map_err(|e| Status::internal(e.to_string()))?;

        // detach handle so it stays alive
        tokio::spawn(async move {
            let _ = handle;
        });
        Ok(Response::new(ConnectionId { id }))
    }

    async fn write(&self, req: Request<WriteRequest>) -> Result<Response<Empty>, Status> {
        let msg = req.into_inner();
        let written = self
            .mgr
            .write_bytes(&msg.id, &msg.data)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        if written != msg.data.len() {
            return Err(Status::internal("short write"));
        }
        Ok(Response::new(Empty {}))
    }

    async fn stop(&self, req: Request<ConnectionId>) -> Result<Response<Empty>, Status> {
        self.mgr
            .stop_connection(&req.into_inner().id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    type ReadStreamStream = tokio_stream::wrappers::ReceiverStream<Result<ByteChunk, Status>>;

    async fn read_stream(
        &self,
        req: Request<ConnectionId>,
    ) -> Result<Response<Self::ReadStreamStream>, Status> {
        let id = req.into_inner().id;
        // create a new channel dedicated to this stream
        let (_chunk_tx, chunk_rx) = mpsc::channel::<Result<ByteChunk, Status>>(32);
        // attach a listener to the echo path
        let mgr = self.mgr.clone();
        tokio::spawn(async move {
            // naive polling loop; you can wire echo_tx later
            let _buf = vec![0u8; 1024];
            loop {
                match mgr.write_bytes(&id, &[]).await {
                    _ => {}
                } // keep handle alive
                  // fake: real impl should hook into echo_tx
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            chunk_rx,
        )))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let mgr = ConnectionManager::new();
    let svc = terminal_server::TerminalServer::new(TerminalSvc::new(mgr));

    info!("gRPC server listening on 0.0.0.0:50051");
    tonic::transport::Server::builder()
        .add_service(svc)
        .serve(([0, 0, 0, 0], 50051).into())
        .await?;

    Ok(())
}
