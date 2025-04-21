use std::env;
use std::process::Command;

fn checkout_submodule() {
    let output = Command::new("git")
        .args(["submodule", "status"])
        .output()
        .expect("Failed to exec `git submodule status`");

    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    if stdout.lines().any(|line| line.starts_with('-')) {
        println!("Initializing v8 submodule...");
        Command::new("git")
            .args(["submodule", "update", "--init"])
            .output()
            .expect("Failed to checkout v8");
    }
}

fn generate_bindings() {}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    checkout_submodule();

    generate_bindings();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=v8_monolith");
}
