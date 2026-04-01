use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-env-changed=RONG_GIT_REVISION");

    if let Ok(value) = std::env::var("RONG_GIT_REVISION") {
        println!("cargo:rustc-env=RONG_GIT_REVISION={value}");
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let output = Command::new("git")
        .arg("-C")
        .arg(&manifest_dir)
        .arg("rev-parse")
        .arg("HEAD")
        .output();

    if let Ok(output) = output
        && output.status.success()
    {
        let revision = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !revision.is_empty() {
            println!("cargo:rustc-env=RONG_GIT_REVISION={revision}");
        }
    }
}
