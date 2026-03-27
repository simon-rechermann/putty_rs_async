//! Library façade for the gRPC-Web server.
//
//  putty_rs_web (and any other crate) only needs `putty_grpc_server::run()`.
//  Internally we keep the implementation in a separate module (`server.rs`)
//  so that both the library and the standalone binary can reuse it.

/// Generated protobuf types (`tonic-build` writes them into OUT_DIR).
pub mod proto {
    tonic::include_proto!("putty_interface");
}
pub use proto as putty_interface;

mod convert;
mod server;

pub use server::run;
