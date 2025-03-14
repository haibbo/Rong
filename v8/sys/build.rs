use std::env;

fn is_debug() -> bool {
    match env::var("PROFILE").unwrap().as_str() {
        "debug" => true,
        "release" => false,
        profile => panic!("Unknown profile: {}", profile),
    }
}

fn generate_bindings() {}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    generate_bindings();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=v8_monolith");
}
