use std::env;
use std::path::Path;
use std::process::Command;

fn checkout_submodule() {
    let output = Command::new("git")
        .args(["submodule", "status"])
        .output()
        .expect("Failed to exec `git submodule status`");

    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    if stdout.lines().any(|line| line.starts_with('-')) {
        Command::new("git")
            .args(["submodule", "update", "--init"])
            .output()
            .expect("Failed to checkout quickjs-ng");
    }
}

fn android_setup() {
    let ndk = env::var("ANDROID_NDK_HOME").expect("ANDROID_NDK_HOME is not set!");
    let arch = env::consts::ARCH;
    let os = if env::consts::OS == "macos" {
        "Darwin"
    } else {
        env::consts::OS
    };
    let api = env::var("API").unwrap_or("22".to_string());

    env::set_var(
        "CC",
        format!("{ndk}/toolchains/llvm/prebuilt/{os}-{arch}/bin/clang -target aarch64-linux-android{api}")
    );

    env::set_var(
        "AR",
        format!("{ndk}/toolchains/llvm/prebuilt/{os}-{arch}/bin/llvm-ar"),
    );

    // for bindgen
    env::set_var(
        "BINDGEN_EXTRA_CLANG_ARGS",
        format!("--sysroot {ndk}/toolchains/llvm/prebuilt/{os}-{arch}/sysroot"),
    );
}

fn build_static_archive() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    if target_os == "android" {
        android_setup();
    }

    let output = Command::new("make")
        .args(["-j4"])
        .output()
        .expect("Failed to execute make");

    assert!(
        output.status.success(),
        "Make failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn generate_binding(out_dir: &String) {
    let builder = bindgen::Builder::default()
        .header("quickjs.wrapper.h")
        .clang_arg("-I./quickjs-ng")
        .clang_arg("-I./patch")
        .allowlist_var("JS_.*")
        .blocklist_type("JSClassID")
        .blocklist_type("JS_MarkFunc")
        .blocklist_type("JSClass")
        .blocklist_type("JSClassDef")
        .blocklist_type("JSClassGCMark")
        .blocklist_type("JSCFunctionListEntry.*")
        .blocklist_type("JSClassExoticMethods")
        .opaque_type("FILE")
        .blocklist_type("FILE")
        .allowlist_function("[Q]*JS_.*")
        .blocklist_function("JS_.*Class.*")
        .blocklist_function("JS_.*Opaque[2]*")
        .blocklist_function("JS_NewCFunc.*")
        .blocklist_function("JS_MarkValue")
        .blocklist_function("JS_.*List")
        .blocklist_function("JS_MarkValue")
        .blocklist_function("JS_DumpMemoryUsage")
        .blocklist_item("JSCFunctionEnum.*")
        .blocklist_item("JSCFunctionType");

    let bindings = builder
        .generate()
        .expect("Unable to generating bingdings for quickjs");

    bindings
        .write_to_file(Path::new(out_dir).join("quickjs.bindings.rs"))
        .expect("Failed to write bindings!");
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    checkout_submodule();
    build_static_archive();
    generate_binding(&out_dir);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Makefile");
    println!("cargo:rerun-if-changed=patch/*");

    // where to find static library libquickjs
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=quickjs");
}
