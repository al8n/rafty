extern crate autocfg;

#[cfg(feature = "async")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .out_dir("src/")
        .compile(&["proto/raft.proto"], &["proto"])
        .unwrap();
    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
