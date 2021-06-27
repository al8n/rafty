fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"kebab-case\")]")
        .out_dir("src/")
        .compile(&["proto/raft.proto"], &["proto"])
        .unwrap();
    Ok(())
}
