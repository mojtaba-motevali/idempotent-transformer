fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .compile(&["proto/workflow_service.proto"], &["proto"])?;

    // Compile FlatBuffers schema if flatc is available; silently skip otherwise.
    // The generated code is only used by the full FlatBuffers codec path.
    // During development the serde_json MVP codec is used instead.
    if let Err(e) = flatbuffers_build::BuilderOptions::new_with_files(["flatbuffers/raft_network.fbs"])
        .compile()
    {
        // Emit a cargo warning so developers know flatc is missing, but do not
        // hard-fail so that `cargo check` succeeds without the FlatBuffers compiler.
        println!("cargo:warning=flatbuffers codegen skipped: {e}");

        // Write an empty generated module so the `include!` in mod.rs compiles.
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let dest = std::path::Path::new(&out_dir).join("raft_network_generated.rs");
        std::fs::write(dest, "pub mod raft_net {}\n")?;
    }

    Ok(())
}
