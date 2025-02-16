use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    let sdk_name = match target_os.as_str() {
        "macos" => "macosx",
        "ios" => "iphoneos",
        other => {
            panic!(
                "cargo:warning=Target OS '{}' does not require JavaScriptCore bindings.",
                other
            );
        }
    };

    let sdk_path_output = Command::new("xcrun")
        .args(["--sdk", sdk_name, "--show-sdk-path"])
        .output()
        .expect("Failed to execute xcrun to get SDK path");

    if !sdk_path_output.status.success() {
        panic!(
            "xcrun failed to get SDK path for SDK '{}': {:?}",
            sdk_name,
            String::from_utf8_lossy(&sdk_path_output.stderr)
        );
    }

    let sdk_path = String::from_utf8(sdk_path_output.stdout)
        .expect("Failed to parse xcrun output as UTF-8")
        .trim()
        .to_string();

    // full path JavaScriptCore.framework/Headers
    let framework_path = "System/Library/Frameworks/JavaScriptCore.framework/Headers";
    let header_path = PathBuf::from(&sdk_path).join(framework_path);
    println!("x{:?}", header_path);

    let header_file = header_path.join("JavaScript.h");

    if !header_file.exists() {
        panic!("Header file not found: {}", header_file.to_string_lossy());
    }

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-lib=framework=JavaScriptCore");

    let bindings = bindgen::Builder::default()
        .header(header_file.to_string_lossy())
        .clang_arg(format!("-I{}", header_path.to_string_lossy()))
        .clang_arg(format!("-isysroot{}", sdk_path))
        .allowlist_function("JS.*")
        .allowlist_type("JS.*")
        .allowlist_var("JS.*")
        .allowlist_var("kJS.*")
        .generate()
        .expect("Unable to generate bindings for JavaScriptCore");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
