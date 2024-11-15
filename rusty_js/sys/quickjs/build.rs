use std::env;
use std::path::PathBuf;
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
            .status()
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

    Command::new("make")
        .args(["-j4"])
        .status()
        .expect("Failed to make make");
}

fn generate_binding(out_dir: &PathBuf) {
    let allow_funcs = vec![
        "JS_NewRuntime",
        "JS_FreeRuntime",
        "JS_NewContext",
        "JS_FreeContext",
        "JS_DupContext",
        "JS_FreeValue",
        "JS_ToCString",
        "JS_FreeCString",
        "JS_ThrowTypeError",
        "JS_ToInt64",
        "QJS_RunScript",
        "QJS_RunJobs",
    ];

    let mut builder = bindgen::Builder::default()
        .header("quickjs-ng/quickjs.h")
        .header("patch/qjs.h");

    for item in &allow_funcs {
        builder = builder.allowlist_function(item);
    }

    let dest = out_dir.join("bindings.rs");

    let bindings = builder.generate().expect("Unable to generating bingdings");
    bindings
        .write_to_file(dest)
        .expect("Failed to write bindings!");
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    checkout_submodule();
    build_static_archive();
    generate_binding(&out_dir);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Makefile");
    println!("cargo:rerun-if-changed=patch/*");

    // where to find static library libquickjs
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=quickjs");
}
