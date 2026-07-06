fn main() -> Result<(), Box<dyn std::error::Error>> {
    // rust-embed requires the folder to exist at compile time; the web UI
    // may not have been built yet (e.g. plain `cargo build`).
    std::fs::create_dir_all("web/dist")?;

    tonic_build::configure()
        .out_dir("src/api")
        .file_descriptor_set_path("src/api/descriptor.bin")
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(
            &[
                "proto/objects/v1alpha1/category.proto",
                "proto/objects/v1alpha1/station.proto",
                "proto/tunein/v1alpha1/browse.proto",
                "proto/tunein/v1alpha1/playback.proto",
            ],
            &["proto"],
        )?;
    Ok(())
}
