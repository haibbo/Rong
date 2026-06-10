use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // cfgs we may emit, declared so rustc's unexpected_cfgs lint stays quiet.
    println!("cargo:rustc-check-cfg=cfg(jsc_system)");
    println!("cargo:rustc-check-cfg=cfg(jsc_source)");
    // Re-run when the pinned prebuilt-artifact manifest changes.
    println!("cargo:rerun-if-changed=webkit-artifacts.tsv");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let has_system_jsc = matches!(target_os.as_str(), "macos" | "ios");

    // Backend choice:
    //   * `source` feature forces the JSCOnly/source backend even on Apple.
    //   * macOS/iOS without the feature use the system JavaScriptCore.framework.
    //   * Everywhere else uses a source-built WebKit/JSC artifact.
    let use_source = env::var_os("CARGO_FEATURE_SOURCE").is_some() || !has_system_jsc;

    if use_source {
        build_source(&target_os);
    } else {
        build_system(&target_os);
    }
}

/// Generate `bindings.rs` from the public JavaScriptCore C API.
///
/// Both backends call this with the SAME allowlist, so the generated symbols,
/// i.e. this crate's entire public surface, are identical regardless of where
/// the library came from. That is what keeps the upper layers backend-agnostic.
fn generate_bindings(header_file: &Path, clang_args: &[String]) {
    let mut builder = bindgen::Builder::default()
        .header(header_file.to_string_lossy())
        .allowlist_function("JS.*")
        .allowlist_type("JS.*")
        .allowlist_var("JS.*")
        .allowlist_var("kJS.*");
    for arg in clang_args {
        builder = builder.clang_arg(arg);
    }
    let bindings = builder
        .generate()
        .expect("Unable to generate bindings for JavaScriptCore");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

/// Compile the C++ bytecode bridge (source backend only).
///
/// The bridge is ALWAYS compiled and linked for the source backend, so its
/// `rong_jsc_*` symbols are always defined and gating the Rust side on
/// `jsc_source` alone never produces an undefined-symbol link error. When the
/// artifact ships JSC's private/internal headers under
/// `<cache>/include/JavaScriptCore/private/JavaScriptCore/` plus the transitive
/// WTF/bmalloc headers, the real implementation is built
/// (`-DRONG_JSC_HAVE_PRIVATE_HEADERS`). Release artifacts must include those
/// headers; set `RONG_JSC_REQUIRE_BYTECODE=1` to make missing bytecode support a
/// build error.
fn compile_bytecode_bridge(include_dir: &Path, target_os: &str, target: &str) {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let bridge_src = manifest_dir.join("src").join("bytecode_bridge.cpp");

    if !bridge_src.exists() {
        return;
    }
    println!("cargo:rerun-if-changed={}", bridge_src.display());

    let private_jsc = include_dir.join("JavaScriptCore").join("private");
    let nested_private_jsc = private_jsc.join("JavaScriptCore");
    let wtf_dir = include_dir.join("WTF");
    let bmalloc_dir = include_dir.join("bmalloc");
    let have_nested_private_headers = nested_private_jsc.is_dir()
        && nested_private_jsc.join("Completion.h").exists()
        && nested_private_jsc.join("BytecodeCacheError.h").exists();
    let have_flat_private_headers = private_jsc.is_dir()
        && private_jsc.join("Completion.h").exists()
        && private_jsc.join("BytecodeCacheError.h").exists();
    let have_private_headers = (have_nested_private_headers || have_flat_private_headers)
        && wtf_dir.is_dir()
        && bmalloc_dir.is_dir();

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(&bridge_src)
        // public:  #include <JavaScriptCore/JavaScript.h>
        .include(include_dir)
        // public (alternate):  #include "JSContextRef.h"
        .include(include_dir.join("JavaScriptCore"))
        .warnings(false);

    // Current WebKit private headers use C++23 library features such as
    // `std::unexpected` and `std::to_underlying`. cc-rs does not currently map
    // `c++23` to an MSVC `/std:` spelling, so pass that one explicitly.
    if target.ends_with("-pc-windows-msvc") {
        add_windows_llvm_to_path();
        build.prefer_clang_cl_over_msvc(true);
        build.flag("/std:c++latest");
        // WebKit's Windows port is normally built with clang-cl, whose
        // preprocessor defines these GCC-compatible target macros. Plain cl.exe
        // does not, but installed headers such as PlatformCPU.h still test them.
        if target.starts_with("x86_64-") || target.starts_with("aarch64-") {
            build.define("__SIZEOF_POINTER__", Some("8"));
        } else if target.starts_with("i686-") || target.starts_with("i586-") {
            build.define("__SIZEOF_POINTER__", Some("4"));
        }
        build
            .define("__ORDER_LITTLE_ENDIAN__", Some("1234"))
            .define("__ORDER_BIG_ENDIAN__", Some("4321"))
            .define("__ORDER_PDP_ENDIAN__", Some("3412"))
            .define("__BYTE_ORDER__", Some("__ORDER_LITTLE_ENDIAN__"))
            .define("STATICALLY_LINKED_WITH_JavaScriptCore", None)
            .define("STATICALLY_LINKED_WITH_WTF", None)
            .define("NOMINMAX", None)
            .define("WIN32_LEAN_AND_MEAN", None);
    } else {
        build.std("c++23");
    }
    // Prebuilt/source artifacts are Release WebKit builds. Keep the bridge's
    // view of WebKit private headers in release mode even when Rust is building
    // debug tests; otherwise private inline helpers reference debug-only JSC
    // symbols that are not shipped in the release static archives.
    build.define("NDEBUG", None);
    if env::var("PROFILE").as_deref() != Ok("release") {
        build.define("RELEASE_WITHOUT_OPTIMIZATIONS", None);
    }

    // Match how WebKit compiles JSC (exceptions + RTTI off). `flag_if_supported`
    // probes each flag and drops it where the compiler rejects it, so the same
    // call set covers gcc/clang (`-fno-*`), clang-cl (accepts both), and a bare
    // MSVC `cl.exe` fallback (`/GR-`, `/EHs-c-`). WebKit's private headers
    // realistically require clang/clang-cl regardless.
    for flag in ["-fno-exceptions", "-fno-rtti", "/GR-", "/EHs-c-"] {
        build.flag_if_supported(flag);
    }

    if have_private_headers {
        build
            // Selects the real implementation in bytecode_bridge.cpp.
            .define("RONG_JSC_HAVE_PRIVATE_HEADERS", None)
            // private:  #include <JavaScriptCore/VM.h>, etc. New source
            // artifacts store these under private/JavaScriptCore; flat-only
            // local artifacts still work through bytecode_bridge.cpp fallback.
            .include(&private_jsc);
        // WTF/bmalloc headers are pulled transitively by JSC private headers.
        build.include(&wtf_dir);
        build.include(&bmalloc_dir);
        let icu_dir = include_dir.join("icu");
        if icu_dir.is_dir() {
            build.include(icu_dir);
        }
    } else if env::var("RONG_JSC_REQUIRE_BYTECODE").as_deref() == Ok("1") {
        panic!(
            "source JavaScriptCore artifact is incomplete: expected private headers at {} \
             (or legacy flat headers at {}) plus WTF and bmalloc headers under {}. Build or \
             download a full JSCOnly artifact with bytecode support.",
            nested_private_jsc.display(),
            private_jsc.display(),
            include_dir.display()
        );
    } else {
        println!(
            "cargo:warning=rong_jscore_sys: JSC private headers not found at {}; \
             building the bytecode bridge stub (bytecode will be unsupported). \
             Set RONG_JSC_REQUIRE_BYTECODE=1 to reject this artifact.",
            nested_private_jsc.display()
        );
    }

    if matches!(target_os, "macos" | "ios" | "tvos" | "watchos") {
        let sdk_name = match target_os {
            "ios" | "tvos" | "watchos" => target_os,
            _ => "macosx",
        };
        if let Ok(output) = std::process::Command::new("xcrun")
            .args(["--sdk", sdk_name, "--show-sdk-path"])
            .output()
            && output.status.success()
            && let Ok(sdk_path) = String::from_utf8(output.stdout)
        {
            build.flag("-isysroot").flag(sdk_path.trim());
        }
    }

    build.compile("jsc_bytecode_bridge");
}

fn add_windows_llvm_to_path() {
    let llvm_bin = Path::new(r"C:\Program Files\LLVM\bin");
    if !llvm_bin.join("clang-cl.exe").exists() {
        return;
    }

    let old_path = env::var_os("PATH").unwrap_or_default();
    let already_present = env::split_paths(&old_path).any(|path| path == llvm_bin);
    if already_present {
        return;
    }

    let mut paths = vec![llvm_bin.to_path_buf()];
    paths.extend(env::split_paths(&old_path));
    if let Ok(new_path) = env::join_paths(paths) {
        unsafe {
            env::set_var("PATH", new_path);
        }
    }
}

/// Resolve the Apple SDK path via `xcrun` for the given target OS.
fn apple_sdk_path(target_os: &str) -> String {
    let sdk_name = match target_os {
        "macos" => "macosx",
        "ios" => "iphoneos",
        "tvos" => "appletvos",
        "watchos" => "watchos",
        other => panic!("no Apple SDK mapping for target OS '{other}'"),
    };

    let output = Command::new("xcrun")
        .args(["--sdk", sdk_name, "--show-sdk-path"])
        .output()
        .expect("Failed to execute xcrun to get SDK path");
    if !output.status.success() {
        panic!(
            "xcrun failed to get SDK path for SDK '{}': {:?}",
            sdk_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout)
        .expect("Failed to parse xcrun output as UTF-8")
        .trim()
        .to_string()
}

/// System backend: link the OS-provided JavaScriptCore.framework (Apple only).
/// Public C API surface only; behavior is unchanged from the original script.
fn build_system(target_os: &str) {
    if !matches!(target_os, "macos" | "ios") {
        panic!("system JavaScriptCore backend is not available for target OS '{target_os}'");
    }
    let sdk_path = apple_sdk_path(target_os);

    let framework_path = "System/Library/Frameworks/JavaScriptCore.framework/Headers";
    let header_path = PathBuf::from(&sdk_path).join(framework_path);
    let header_file = header_path.join("JavaScript.h");
    if !header_file.exists() {
        panic!("Header file not found: {}", header_file.to_string_lossy());
    }

    generate_bindings(
        &header_file,
        &[
            format!("-I{}", header_path.to_string_lossy()),
            "-isysroot".to_string(),
            sdk_path,
        ],
    );

    println!("cargo:rustc-link-lib=framework=JavaScriptCore");
    println!("cargo:rustc-cfg=jsc_system");
    emit_metadata("backend", "system");
}

/// Source backend: link a JSCOnly/source-built WebKit artifact.
fn build_source(target_os: &str) {
    println!("cargo:rerun-if-env-changed=RONG_JSC_REQUIRE_BYTECODE");
    let target = env::var("TARGET").unwrap();
    let source = resolve_source_layout(&target);
    let header_file = source.include.join("JavaScriptCore").join("JavaScript.h");

    let mut clang_args = vec![format!("-I{}", source.include.display())];
    if matches!(target_os, "macos" | "ios" | "tvos" | "watchos") {
        clang_args.push("-isysroot".to_string());
        clang_args.push(apple_sdk_path(target_os));
    }
    generate_bindings(&header_file, &clang_args);

    // Compile the C++ bytecode bridge (source backend only — the system
    // framework doesn't provide the private headers needed).
    compile_bytecode_bridge(&source.include, target_os, &target);

    // A vanilla JSCOnly build on Apple produces a `JavaScriptCore.framework`
    // (dynamic) rather than the static `.a` set; link it as a framework. The
    // artifact is relocatable, so stamp the local extracted framework with its
    // current absolute install name before consumers link against it.
    let framework_dir = source.lib.join("JavaScriptCore.framework");
    if framework_dir.is_dir() {
        stamp_apple_framework_install_name(&framework_dir);
        println!("cargo:rustc-link-search=framework={}", source.lib.display());
        println!("cargo:rustc-link-lib=framework=JavaScriptCore");
        if matches!(target_os, "macos" | "ios" | "tvos" | "watchos") {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", source.lib.display());
        }
    } else {
        println!("cargo:rustc-link-search=native={}", source.lib.display());
        for lib in link_libs(target_os) {
            println!("cargo:rustc-link-lib={lib}");
        }
    }

    println!("cargo:rustc-cfg=jsc_source");
    emit_metadata("backend", "source");
    emit_metadata("include", &source.include.display().to_string());
    emit_metadata("lib", &source.lib.display().to_string());
    if let Some(webkit_revision) = &source.webkit_revision {
        emit_metadata("webkit_revision", webkit_revision);
    }
}

fn stamp_apple_framework_install_name(framework_dir: &Path) {
    let binary = framework_dir
        .join("Versions")
        .read_dir()
        .ok()
        .and_then(|entries| {
            entries.filter_map(Result::ok).find_map(|entry| {
                let candidate = entry.path().join("JavaScriptCore");
                candidate.is_file().then_some(candidate)
            })
        })
        .or_else(|| {
            let candidate = framework_dir.join("JavaScriptCore");
            candidate.is_file().then_some(candidate)
        });
    let Some(binary) = binary else {
        println!(
            "cargo:warning=rong_jscore_sys: could not find JavaScriptCore framework binary under {}",
            framework_dir.display()
        );
        return;
    };

    let status = Command::new("install_name_tool")
        .arg("-id")
        .arg(&binary)
        .arg(&binary)
        .status();
    match status {
        Ok(status) if status.success() => {}
        Ok(status) => {
            println!("cargo:warning=rong_jscore_sys: install_name_tool failed with status {status}")
        }
        Err(err) => {
            println!("cargo:warning=rong_jscore_sys: could not run install_name_tool: {err}")
        }
    }
}

struct SourceLayout {
    include: PathBuf,
    lib: PathBuf,
    webkit_revision: Option<String>,
}

enum SourceOrigin {
    Env,
    PinnedArtifact,
}

fn resolve_source_layout(target: &str) -> SourceLayout {
    // Resolution order — one manual override, then fully automatic:
    //   1. RONG_JSC_ROOT — an install tree (`include/` + `lib/`) or a WebKit
    //      build tree (`WebKitBuild/JSCOnly/Release`). Per-target variants such
    //      as `RONG_JSC_ROOT_AARCH64_UNKNOWN_LINUX_GNU` are also honored.
    //   2. The shared per-target cache (`~/.cache/rong/webkit/<target>`).
    //   3. A pinned prebuilt artifact downloaded into that cache.
    let mut root = target_env_path("RONG_JSC_ROOT", target)
        .map(|root| (root, SourceOrigin::Env))
        .or_else(|| {
            default_cache_artifact(target).map(|root| (root, SourceOrigin::PinnedArtifact))
        });

    // No artifact anywhere: try the pinned prebuilt artifact. This is the
    // cross-platform path for Windows/Linux and keeps Cargo builds lightweight.
    if root.is_none() {
        maybe_download(target);
        root = default_cache_artifact(target).map(|root| (root, SourceOrigin::PinnedArtifact));
    }

    let Some((root, origin)) = root else {
        panic!(
            "source JavaScriptCore backend needs an artifact for {target}. Options: let the build \
             download a pinned prebuilt one (a release must publish it; see webkit-artifacts.tsv), \
             or point RONG_JSC_ROOT at a JSCOnly install tree (include/ + lib/) or build tree \
             (WebKitBuild/JSCOnly/Release). See javascriptcore/sys/README.md."
        )
    };

    let configs = ["Release", "Debug"];
    let mut candidates = vec![
        (root.join("include"), root.join("lib")),
        (root.join("usr/include"), root.join("usr/lib")),
    ];
    for config in configs {
        candidates.push((
            root.join(format!("WebKitBuild/JSCOnly/{config}/include")),
            root.join(format!("WebKitBuild/JSCOnly/{config}/lib")),
        ));
        candidates.push((
            root.join(format!("WebKitBuild/{config}/include")),
            root.join(format!("WebKitBuild/{config}/lib")),
        ));
    }

    for (include, lib) in candidates {
        if include.join("JavaScriptCore/JavaScript.h").exists() && lib.is_dir() {
            let webkit_revision = match origin {
                SourceOrigin::PinnedArtifact => artifact_for(target)
                    .and_then(|artifact| webkit_revision_from_tag(&artifact.tag))
                    .map(|revision| format!("webkit-{revision}")),
                SourceOrigin::Env => None,
            };
            return SourceLayout {
                include,
                lib,
                webkit_revision,
            };
        }
    }

    panic!(
        "could not find JavaScriptCore headers/libs under {}; expected include/JavaScriptCore/JavaScript.h \
         and lib/.",
        root.display()
    );
}

/// The per-target artifact directory under the shared cache (whether or not it
/// is populated yet). Layout: `<cache>/<target-triple>/{include,lib}`.
/// `<cache>` is `$RONG_JSC_CACHE_DIR`, else `$XDG_CACHE_HOME/rong/webkit`, else
/// `$HOME/.cache/rong/webkit` (`%USERPROFILE%\.cache\rong\webkit` on Windows).
fn cache_target_dir(target: &str) -> Option<PathBuf> {
    println!("cargo:rerun-if-env-changed=RONG_JSC_CACHE_DIR");
    let mut base = if let Some(dir) = env::var_os("RONG_JSC_CACHE_DIR") {
        PathBuf::from(dir)
    } else if let Some(xdg) = env::var_os("XDG_CACHE_HOME") {
        PathBuf::from(xdg).join("rong").join("webkit")
    } else {
        let home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"))?;
        PathBuf::from(home)
            .join(".cache")
            .join("rong")
            .join("webkit")
    };
    if base.is_relative() {
        base = env::current_dir().ok()?.join(base);
    }
    Some(base.join(target))
}

/// The per-target cache artifact, but only if it is actually populated.
fn default_cache_artifact(target: &str) -> Option<PathBuf> {
    let artifact = cache_target_dir(target)?;
    artifact
        .join("include/JavaScriptCore/JavaScript.h")
        .exists()
        .then_some(artifact)
}

/// Default host for prebuilt artifacts (GitHub release assets of this repo).
/// The full URL is `<base>/<tag>/<file>`; override the base with
/// `RONG_JSC_ARTIFACT_BASE_URL` for mirrors or air-gapped setups.
const DEFAULT_ARTIFACT_BASE_URL: &str = "https://github.com/LingXia-Dev/Rong/releases/download";

struct Artifact {
    tag: String,
    file: String,
    sha256: String,
}

/// Look up the pinned prebuilt artifact for `target` in `webkit-artifacts.tsv`.
///
/// The manifest is embedded at build time (`include_str!`) so this also works
/// for crates.io consumers. Format per row (whitespace-separated):
///   `<target-triple> <release-tag> <filename> <sha256-hex>`
/// Lines that are blank or start with `#` are ignored.
fn artifact_for(target: &str) -> Option<Artifact> {
    const MANIFEST: &str = include_str!("webkit-artifacts.tsv");
    for line in MANIFEST.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut cols = line.split_whitespace();
        let row_target = cols.next()?;
        if row_target != target {
            continue;
        }
        let tag = cols.next()?.to_string();
        let file = cols.next()?.to_string();
        let sha256 = cols.next()?.to_string();
        return Some(Artifact { tag, file, sha256 });
    }
    None
}

fn webkit_revision_from_tag(tag: &str) -> Option<String> {
    tag.strip_prefix("jsc-artifacts-webkit-")
        .and_then(|rest| rest.split_once('-').map(|(revision, _)| revision))
        .filter(|revision| !revision.is_empty())
        .map(str::to_string)
}

/// Download + verify + extract a pinned prebuilt JSCOnly artifact into the
/// per-target cache. Cross-platform: shells out to `curl` (or `wget`) and `tar`,
/// which ship with macOS, Windows 10 1803+, and essentially all Linux. Pure
/// best-effort — any failure logs a warning and returns so the caller can emit
/// one clear "artifact missing" panic. Opt out with `RONG_JSC_DOWNLOAD=0`.
fn maybe_download(target: &str) {
    println!("cargo:rerun-if-env-changed=RONG_JSC_DOWNLOAD");
    println!("cargo:rerun-if-env-changed=RONG_JSC_ARTIFACT_BASE_URL");
    if env::var("RONG_JSC_DOWNLOAD").as_deref() == Ok("0") {
        return;
    }
    let Some(artifact) = artifact_for(target) else {
        // No prebuilt artifact pinned for this target yet — fall through.
        return;
    };
    let Some(dest) = cache_target_dir(target) else {
        return;
    };

    let base = env::var("RONG_JSC_ARTIFACT_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_ARTIFACT_BASE_URL.to_string());
    let url = format!(
        "{}/{}/{}",
        base.trim_end_matches('/'),
        artifact.tag,
        artifact.file,
    );

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let tmp = out_dir.join(&artifact.file);

    println!(
        "cargo:warning=rong_jscore: downloading prebuilt JSCOnly artifact for {target} from {url}"
    );

    if let Err(e) = download_file(&url, &tmp) {
        println!(
            "cargo:warning=rong_jscore: artifact download failed ({e}); ignoring prebuilt artifact"
        );
        let _ = std::fs::remove_file(&tmp);
        return;
    }

    match std::fs::read(&tmp) {
        Ok(bytes) => {
            let got = sha256::hex(&bytes);
            if !got.eq_ignore_ascii_case(&artifact.sha256) {
                println!(
                    "cargo:warning=rong_jscore: artifact checksum mismatch for {} (expected {}, got {}); \
                     ignoring download",
                    artifact.file, artifact.sha256, got
                );
                let _ = std::fs::remove_file(&tmp);
                return;
            }
        }
        Err(e) => {
            println!("cargo:warning=rong_jscore: could not read downloaded artifact ({e})");
            let _ = std::fs::remove_file(&tmp);
            return;
        }
    }

    // Extract `include/` and `lib/` into <cache>/<target>/.
    if let Err(e) = std::fs::create_dir_all(&dest) {
        println!(
            "cargo:warning=rong_jscore: could not create cache dir {} ({e})",
            dest.display()
        );
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    if let Err(e) = extract_tar_gz(&tmp, &dest) {
        println!(
            "cargo:warning=rong_jscore: artifact extraction failed ({e}); ignoring prebuilt artifact"
        );
        // A partial extraction can leave `dest` with just enough (e.g.
        // JavaScript.h) to satisfy the cache-hit check in
        // `default_cache_artifact`, which would then feed a truncated artifact
        // to a later build and fail with obscure missing-header/linker errors.
        // Remove it so the next build re-downloads cleanly.
        let _ = std::fs::remove_dir_all(&dest);
    }
    let _ = std::fs::remove_file(&tmp);
}

/// Download `url` to `dest` using `curl`, falling back to `wget`.
fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    // curl: -f fail on HTTP error, -S show errors, -L follow redirects
    // (GitHub release assets 302 to a CDN), --retry for flaky networks/CDN 504s.
    let _ = std::fs::remove_file(dest);
    let curl = Command::new("curl")
        .args([
            "-fSL",
            "--retry",
            "8",
            "--retry-delay",
            "10",
            "--connect-timeout",
            "30",
            "--max-time",
            "900",
            "-o",
        ])
        .arg(dest)
        .arg(url)
        .status();
    match curl {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            let _ = std::fs::remove_file(dest);
            match download_file_with_wget(url, dest) {
                Ok(()) => Ok(()),
                Err(err) => Err(format!("curl exited with {s}; {err}")),
            }
        }
        Err(err) => {
            let _ = std::fs::remove_file(dest);
            match download_file_with_wget(url, dest) {
                Ok(()) => Ok(()),
                Err(wget_err) => Err(format!("could not run curl: {err}; {wget_err}")),
            }
        }
    }
}

fn download_file_with_wget(url: &str, dest: &Path) -> Result<(), String> {
    let wget = Command::new("wget")
        .args(["--tries=8", "--waitretry=10", "--timeout=30", "-O"])
        .arg(dest)
        .arg(url)
        .status();
    match wget {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("wget exited with {s}")),
        Err(err) => Err(format!("could not run wget: {err}")),
    }
}

/// Extract a `.tar.gz` into `dest` using the system `tar` (bsdtar on
/// macOS/Windows, GNU tar on Linux — all accept `-xzf`).
fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), String> {
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(archive)
        .arg("-C")
        .arg(dest)
        .status();
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("tar exited with {s}")),
        Err(e) => Err(format!("could not run tar: {e}")),
    }
}

/// Minimal, dependency-free SHA-256 used to verify downloaded artifacts.
mod sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    /// Hex-encoded SHA-256 of `data`.
    pub fn hex(data: &[u8]) -> String {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];

        // Pad: append 0x80, then zeros, then the 64-bit big-endian bit length.
        let bit_len = (data.len() as u64).wrapping_mul(8);
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());

        for chunk in msg.chunks_exact(64) {
            let mut w = [0u32; 64];
            for (i, word) in w.iter_mut().take(16).enumerate() {
                *word = u32::from_be_bytes([
                    chunk[i * 4],
                    chunk[i * 4 + 1],
                    chunk[i * 4 + 2],
                    chunk[i * 4 + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }

            let mut v = h;
            for i in 0..64 {
                let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
                let ch = (v[4] & v[5]) ^ ((!v[4]) & v[6]);
                let t1 = v[7]
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
                let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
                let t2 = s0.wrapping_add(maj);
                v[7] = v[6];
                v[6] = v[5];
                v[5] = v[4];
                v[4] = v[3].wrapping_add(t1);
                v[3] = v[2];
                v[2] = v[1];
                v[1] = v[0];
                v[0] = t1.wrapping_add(t2);
            }
            for (hi, vi) in h.iter_mut().zip(v.iter()) {
                *hi = hi.wrapping_add(*vi);
            }
        }

        let mut out = String::with_capacity(64);
        for word in h {
            out.push_str(&format!("{word:08x}"));
        }
        out
    }
}

fn link_libs(target_os: &str) -> Vec<String> {
    let mut libs = vec![
        static_archive_lib(target_os, "JavaScriptCore"),
        static_archive_lib(target_os, "WTF"),
        static_archive_lib(target_os, "bmalloc"),
    ];

    match target_os {
        "macos" | "ios" | "tvos" | "watchos" => {
            libs.extend([
                "dylib=icucore".to_string(),
                "framework=CoreFoundation".to_string(),
                "dylib=c++".to_string(),
            ]);
        }
        "linux" | "android" => {
            libs.extend([
                static_dep_lib(target_os, "icui18n"),
                static_dep_lib(target_os, "icuuc"),
                static_dep_lib(target_os, "icudata"),
                "dylib=pthread".to_string(),
            ]);
            if target_os == "linux" {
                libs.extend([
                    "dylib=dl".to_string(),
                    "dylib=m".to_string(),
                    "dylib=atomic".to_string(),
                ]);
            } else {
                libs.push("dylib=c++".to_string());
            }
        }
        "windows" => {
            libs.extend([
                "static=sicuin".to_string(),
                "static=sicuuc".to_string(),
                "static=sicudt".to_string(),
                "dylib=advapi32".to_string(),
                "dylib=shell32".to_string(),
                "dylib=winmm".to_string(),
            ]);
        }
        other => panic!("source backend: unsupported target OS '{other}'"),
    }
    libs
}

fn static_archive_lib(target_os: &str, name: &str) -> String {
    if matches!(target_os, "linux" | "android") {
        format!("static:+whole-archive,-bundle={name}")
    } else {
        format!("static={name}")
    }
}

fn static_dep_lib(target_os: &str, name: &str) -> String {
    if matches!(target_os, "linux" | "android") {
        format!("static:-bundle={name}")
    } else {
        format!("static={name}")
    }
}

fn target_env_path(base: &str, target: &str) -> Option<PathBuf> {
    target_env(base, target).map(PathBuf::from)
}

fn target_env(base: &str, target: &str) -> Option<String> {
    let target_key = format!("{base}_{}", target_env_suffix(target));
    println!("cargo:rerun-if-env-changed={target_key}");
    println!("cargo:rerun-if-env-changed={base}");
    env::var(&target_key).ok().or_else(|| env::var(base).ok())
}

fn target_env_suffix(target: &str) -> String {
    target.replace(['-', '.'], "_").to_ascii_uppercase()
}

fn emit_metadata(key: &str, value: &str) {
    // `cargo::metadata=KEY=VALUE` (note the double colon) is what surfaces to
    // dependents as `DEP_JSCORE_<KEY>`. The single-colon `cargo:metadata=...`
    // form would instead expose `DEP_JSCORE_METADATA`, which nothing reads.
    println!("cargo::metadata={key}={value}");
}
