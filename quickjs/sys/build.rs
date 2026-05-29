use std::env;
use std::path::{Path, PathBuf};
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

fn harmony_setup(build: &mut cc::Build) {
    let ndk = env::var("OHOS_NDK_HOME").expect("OHOS_NDK_HOME is not set!");

    build.compiler(format!(
        "{ndk}/native/llvm/bin/aarch64-unknown-linux-ohos-clang"
    ));
    build.archiver(format!("{ndk}/native/llvm/bin/llvm-ar"));

    unsafe {
        env::set_var(
            "BINDGEN_EXTRA_CLANG_ARGS",
            format!("--sysroot {ndk}/native/sysroot"),
        );
    }
}

fn android_setup(build: &mut cc::Build) {
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

    build.compiler(format!(
        "{ndk}/toolchains/llvm/prebuilt/{os}-{arch}/bin/clang"
    ));
    build.archiver(format!(
        "{ndk}/toolchains/llvm/prebuilt/{os}-{arch}/bin/llvm-ar"
    ));
    build.flag("-target");
    build.flag(format!("{cc_target}{api}"));

    unsafe {
        env::set_var(
            "BINDGEN_EXTRA_CLANG_ARGS",
            format!("--sysroot {ndk}/toolchains/llvm/prebuilt/{os}-{arch}/sysroot"),
        );
    }
}

fn ios_setup(build: &mut cc::Build) {
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

    build.compiler(&clang_path);
    build.flag("-isysroot");
    build.flag(&sdk_path);
    build.flag("-arch");
    build.flag("arm64");
    build.flag("-mios-version-min=11.0");

    let ar = Command::new("xcrun")
        .args(["--find", "ar"])
        .output()
        .expect("failed to find ar")
        .stdout;
    let ar_path = String::from_utf8_lossy(&ar).trim().to_string();
    build.archiver(&ar_path);

    unsafe {
        env::set_var(
            "BINDGEN_EXTRA_CLANG_ARGS",
            format!("-isysroot {}", sdk_path),
        );
    }
}

/// Merge quickjs.c + extra.c into a single file
/// extra.c accesses quickjs internals so it must be compiled in the same translation unit.
fn merge_quickjs_source(out_dir: &Path) -> PathBuf {
    let quickjs_c = std::fs::read_to_string("quickjs-ng/quickjs.c")
        .expect("Failed to read quickjs-ng/quickjs.c");
    let extra_c = std::fs::read_to_string("patch/extra.c").expect("Failed to read patch/extra.c");

    let merged_path = out_dir.join("new_quickjs.c");
    std::fs::write(&merged_path, format!("{quickjs_c}\n{extra_c}"))
        .expect("Failed to write merged quickjs source");
    merged_path
}

fn build_quickjs(out_dir: &Path) {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let profile = env::var("PROFILE").unwrap();

    let mut build = cc::Build::new();

    // Platform-specific setup
    match target_os.as_str() {
        "linux" if target_env == "ohos" => harmony_setup(&mut build),
        "android" => android_setup(&mut build),
        "ios" => ios_setup(&mut build),
        _ => {} // macOS, Windows, etc. — cc crate auto-detects
    }

    // Include paths
    build.include("quickjs-ng");
    build.include("patch");
    build.define("QUICKJS_NG_BUILD", None);
    build.define("_GNU_SOURCE", None);

    // Optimization
    if profile == "release" {
        build.opt_level(2);
        build.define("NDEBUG", None);
    } else {
        build.opt_level(0);
        build.debug(true);
    }

    // Warnings
    build.warnings(true);

    if target_os == "windows" {
        build.define("WIN32_LEAN_AND_MEAN", None);
        build.define("NOMINMAX", None);
    }
    if target_os == "windows" && target_env == "msvc" {
        build.flag_if_supported("/std:c11");
        build.flag_if_supported("/experimental:c11atomics");
        build.flag_if_supported("/J");
    }

    // Merge quickjs.c + extra.c (extra.c accesses quickjs internals)
    let merged = merge_quickjs_source(out_dir);

    // Source files
    build.file(&merged); // quickjs.c + extra.c merged
    build.file("quickjs-ng/libregexp.c");
    build.file("quickjs-ng/libunicode.c");
    build.file("quickjs-ng/dtoa.c");
    build.file("patch/qjs.c");
    build.file("patch/inline.c");

    // Build static library named "quickjs"
    build.compile("quickjs");
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
    build_quickjs(Path::new(&out_dir));
    generate_binding(&out_dir);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=quickjs.wrapper.h");
    // Watch directories so edits/additions don't require updating this list.
    println!("cargo:rerun-if-changed=patch");
    println!("cargo:rerun-if-changed=quickjs-ng");
    println!("cargo:rerun-if-env-changed=NUM_JOBS");
    println!("cargo:rerun-if-env-changed=OHOS_NDK_HOME");
    println!("cargo:rerun-if-env-changed=ANDROID_NDK_ROOT");
    println!("cargo:rerun-if-env-changed=ANDROID_API_LEVEL");
}
