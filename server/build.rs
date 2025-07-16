fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Building proto files");
    tonic_build::configure()
        .build_server(true)
        .compile(&["proto/workflow_service.proto"], &["proto"])?;
    Ok(())
}
