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

fn harmony_setup() {
    let ndk = env::var("OHOS_NDK_HOME").expect("OHOS_NDK_HOME is not set!");
    unsafe {
        env::set_var(
            "CC",
            format!("{ndk}/native/llvm/bin/aarch64-unknown-linux-ohos-clang"),
        );

        env::set_var("AR", format!("{ndk}/native/llvm/bin/llvm-ar"));

        // for bindgen
        env::set_var(
            "BINDGEN_EXTRA_CLANG_ARGS",
            format!("--sysroot {ndk}/native/sysroot"),
        );
    }
}

fn android_setup() {
    let ndk = env::var("ANDROID_NDK_ROOT").expect("ANDROID_NDK_ROOT is not set!");
    let arch = env::consts::ARCH;
    let os = if env::consts::OS == "macos" {
        "Darwin"
    } else {
        env::consts::OS
    };
    let api = env::var("ANDROID_API_LEVEL").unwrap_or("33".to_string());

    let target = env::var("TARGET").unwrap_or_default();
    let cc_target = if target.contains("armv7") {
        "armv7a-linux-androideabi"
    } else {
        "aarch64-linux-android"
    };

    unsafe {
        env::set_var(
            "CC",
            format!(
                "{ndk}/toolchains/llvm/prebuilt/{os}-{arch}/bin/clang -target {cc_target}{api}"
            ),
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
}

// use utility xcrun to get path of sdk and clang
fn ios_setup() {
    let sdk = Command::new("xcrun")
        .args(["--show-sdk-path", "--sdk", "iphoneos"])
        .output()
        .expect("failed to execute xcrun")
        .stdout;
    let sdk_path = String::from_utf8_lossy(&sdk).trim().to_string();

    let clang = Command::new("xcrun")
        .args(["--find", "clang"])
        .output()
        .expect("failed to find clang")
        .stdout;
    let clang_path = String::from_utf8_lossy(&clang).trim().to_string();

    unsafe {
        env::set_var(
            "CC",
            format!(
                "{} -isysroot {} -arch arm64 -mios-version-min=11.0",
                clang_path, sdk_path
            ),
        );

        let ar = Command::new("xcrun")
            .args(["--find", "ar"])
            .output()
            .expect("failed to find ar")
            .stdout;
        let ar_path = String::from_utf8_lossy(&ar).trim().to_string();
        env::set_var("AR", ar_path);

        // extra args for bindgen
        env::set_var(
            "BINDGEN_EXTRA_CLANG_ARGS",
            format!("-isysroot {}", sdk_path),
        );
    }
}

fn build_static_archive(out_dir: &str) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    match target_os.as_str() {
        "linux" => {
            if env::var("CARGO_CFG_TARGET_ENV").unwrap() == "ohos" {
                harmony_setup()
            }
        }
        "android" => android_setup(),
        "ios" => ios_setup(),
        _ => {} // do nothing for other os, like masosx
    }

    // PROFILE(debug or release) is set automatically by cargo
    let profile = env::var("PROFILE").unwrap();
    let jobs = env::var("NUM_JOBS").unwrap_or_else(|_| "4".to_string());

    let output = Command::new("make")
        .args([
            &format!("-j{jobs}"),
            &format!("PROFILE={profile}"),
            // Ensure build artifacts stay inside Cargo's OUT_DIR to avoid polluting the repo and
            // triggering unnecessary rebuilds.
            &format!("OUT_DIR={out_dir}"),
        ])
        .output()
        .expect("Failed to execute make");

    assert!(
        output.status.success(),
        "Make failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn generate_binding(out_dir: &str) {
    let builder = bindgen::Builder::default()
        .header("quickjs.wrapper.h")
        .clang_arg("-I./quickjs-ng")
        .clang_arg("-I./patch")
        .allowlist_var("JS_.*")
        .blocklist_type("JSClassID")
        .blocklist_type("JSClass")
        .blocklist_type("JSClassDef")
        .blocklist_type("JSCFunctionListEntry.*")
        .blocklist_type("JSClassExoticMethods")
        .opaque_type("FILE")
        .blocklist_type("FILE")
        .allowlist_function("[Q]*JS_.*")
        .blocklist_function("JS_.*Class.*")
        .blocklist_function("JS_GetOpaque")
        .blocklist_function("JS_SetOpaque")
        .blocklist_function("JS_GetOpaque2")
        .blocklist_function("JS_GetAnyOpaque")
        .blocklist_function("JS_NewCFunc.*")
        .blocklist_function("JS_.*List")
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
    build_static_archive(&out_dir);
    generate_binding(&out_dir);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Makefile");
    println!("cargo:rerun-if-changed=quickjs.wrapper.h");
    // Watch directories so edits/additions don't require updating this list.
    println!("cargo:rerun-if-changed=patch");
    println!("cargo:rerun-if-changed=quickjs-ng");
    println!("cargo:rerun-if-env-changed=NUM_JOBS");
    println!("cargo:rerun-if-env-changed=OHOS_NDK_HOME");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_ROOT");
    println!("cargo:rerun-if-env-changed=ANDROID_API_LEVEL");

    // where to find static library libquickjs
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=quickjs");
}
