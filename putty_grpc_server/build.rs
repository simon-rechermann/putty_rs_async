fn main() {
    tonic_build::configure()
        .build_server(true)
        .build_client(false) // we only need the server side
        .out_dir("src/")              // generated code goes into src/
        .compile_protos(&["proto/putty_interface.proto"], &["proto"])
        .unwrap();
}
