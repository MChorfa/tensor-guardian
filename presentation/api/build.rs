use std::io::Result;

fn main() -> Result<()> {
    // Compile protobuf files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/accelerator.proto"], &["proto"])?;

    Ok(())
}
