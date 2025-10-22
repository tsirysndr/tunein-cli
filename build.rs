fn main() -> Result<(), Box<dyn std::error::Error>> {
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
