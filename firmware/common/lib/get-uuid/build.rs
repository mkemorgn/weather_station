use std::{env, fs::File, io::Write, path::Path};
use uuid::Uuid;

fn main() -> anyhow::Result<()> {
    // Tell Cargo to re-run this script if build.rs itself changes:
    println!("cargo:rerun-if-changed=build.rs");

    // Path to write into:
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("uuid.rs");

    // If it already exists, skip re-writing (so UUID is stable):
    if dest_path.exists() {
        return Ok(());
    }

    // Generate a v4 UUID:
    let uuid_val = Uuid::new_v4().to_string();

    // Write the Rust source file:
    let mut f = File::create(&dest_path)?;
    writeln!(f, "/// Auto-generated UUID; stable across rebuilds")?;
    writeln!(f, "pub const UUID: &str = \"{}\";", uuid_val)?;

    Ok(())
}
