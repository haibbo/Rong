#!/usr/bin/env bash
#
# build_jsc_artifact.sh — build a JSCOnly artifact for the rong source backend.
#
# Shallow-clones UPSTREAM WebKit (https://github.com/WebKit/WebKit), builds
# JavaScriptCore in Release, and installs
# a normalized artifact:
#
#   <out>/include/JavaScriptCore/JavaScript.h        (public C API headers)
#   <out>/include/JavaScriptCore/private/            (private headers — bytecode)
#   <out>/include/WTF/  <out>/include/bmalloc/        (transitive private headers)
#   <out>/lib/JavaScriptCore.framework               (Apple)  -- or --
#   <out>/lib/lib{JavaScriptCore,WTF,bmalloc}.a       (non-Apple, static)
#   <out>/lib/lib{icui18n,icuuc,icudata}.a            (non-Apple, static ICU)
#
# That layout is exactly what `rong_jscore_sys/build.rs` auto-detects in the
# per-target cache, so afterwards `RONG_JSC_SOURCE=1 cargo build` needs no env
# var. See javascriptcore/sys/README.md.
#
# NOTE: building WebKit takes a long time and needs full Xcode (macOS) or a
# CMake/clang/ICU/ruby toolchain (Linux). This script is exercised by CI
# (.github/workflows/build-jsc-artifacts.yml); run it locally only if you want a
# local artifact.

set -euo pipefail

WEBKIT_URL="${WEBKIT_URL:-https://github.com/WebKit/WebKit.git}"
# PIN ME: set to a known-good WebKit tag for reproducible artifacts. `main`
# tracks tip-of-tree and is not reproducible; override with --webkit-ref.
DEFAULT_WEBKIT_REF="main"

# Static ICU version built from source for non-Apple targets (distro libicu-dev
# is shared-only). Override with the ICU_VERSION env var.
ICU_VERSION="${ICU_VERSION:-74.2}"

WEBKIT_REF="$DEFAULT_WEBKIT_REF"
WEBKIT_ROOT=""        # reuse an existing checkout instead of cloning
TARGET=""             # default: host triple from rustc
OUT_DIR=""            # default: per-target cache
PACKAGE=""            # if set, also write this .tar.gz
JOBS=""
DRY_RUN=0

usage() {
    cat <<'EOF'
Usage: ./scripts/build_jsc_artifact.sh [options]

Options:
  --webkit-ref <ref>    WebKit tag/branch/sha to build (default: main; pin for releases)
  --webkit-root <path>  Use an existing WebKit checkout (skip clone)
  --target <triple>     Target triple (default: host triple from rustc)
  --out <dir>           Install dir (default: <cache>/<target>; then set RONG_JSC_ROOT=<dir>)
  --package <file.tgz>  Also emit a distributable tarball of the install tree
  --jobs <N>            Parallel build jobs
  --dry-run             Print the plan and exit
  -h, --help            Show this help

The cache base is $RONG_JSC_CACHE_DIR, else $XDG_CACHE_HOME/rong/webkit,
else ~/.cache/rong/webkit.
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --webkit-ref)  WEBKIT_REF="$2"; shift 2 ;;
        --webkit-root) WEBKIT_ROOT="$2"; shift 2 ;;
        --target)      TARGET="$2"; shift 2 ;;
        --out)         OUT_DIR="$2"; shift 2 ;;
        --package)     PACKAGE="$2"; shift 2 ;;
        --jobs)        JOBS="$2"; shift 2 ;;
        --dry-run)     DRY_RUN=1; shift ;;
        -h|--help)     usage; exit 0 ;;
        *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
    esac
done

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# --- Resolve target + output dir -------------------------------------------
if [[ -z "$TARGET" ]]; then
    if ! command -v rustc >/dev/null 2>&1; then
        echo "rustc not found; pass --target <triple>." >&2
        exit 1
    fi
    TARGET="$(rustc -vV | awk '/^host:/{print $2}')"
fi

if [[ -z "$OUT_DIR" ]]; then
    CACHE_BASE="${RONG_JSC_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/rong/webkit}"
    # When RONG_JSC_CACHE_DIR is set it already names the base; otherwise we
    # appended rong/webkit above. build.rs joins <base>/<target>.
    if [[ -n "${RONG_JSC_CACHE_DIR:-}" ]]; then
        OUT_DIR="$RONG_JSC_CACHE_DIR/$TARGET"
    else
        OUT_DIR="$CACHE_BASE/$TARGET"
    fi
fi

case "$(uname -s)" in
    Darwin) IS_DARWIN=1 ;;
    *)      IS_DARWIN=0 ;;
esac

echo "WebKit URL:  $WEBKIT_URL"
echo "WebKit ref:  $WEBKIT_REF"
echo "Target:      $TARGET"
echo "Install to:  $OUT_DIR"
[[ -n "$PACKAGE" ]] && echo "Package:     $PACKAGE"
if [[ "$WEBKIT_REF" == "main" && -z "$WEBKIT_ROOT" ]]; then
    echo "WARNING: building from 'main' (tip-of-tree) is not reproducible; pass --webkit-ref <tag>." >&2
fi
[[ "$DRY_RUN" -eq 1 ]] && exit 0

# --- Obtain WebKit source ---------------------------------------------------
if [[ -z "$WEBKIT_ROOT" ]]; then
    WEBKIT_ROOT="$ROOT_DIR/target/webkit-src"
    if [[ ! -d "$WEBKIT_ROOT/.git" ]]; then
        echo "==> Shallow-cloning WebKit ($WEBKIT_REF)"
        rm -rf "$WEBKIT_ROOT"
        git clone --depth 1 --branch "$WEBKIT_REF" "$WEBKIT_URL" "$WEBKIT_ROOT" 2>/dev/null \
            || git clone --depth 1 "$WEBKIT_URL" "$WEBKIT_ROOT"  # branch arg fails for raw shas
    fi
    # If --webkit-ref is a sha, fetch+checkout it explicitly.
    if ! git -C "$WEBKIT_ROOT" rev-parse --verify --quiet "$WEBKIT_REF^{commit}" >/dev/null; then
        git -C "$WEBKIT_ROOT" fetch --depth 1 origin "$WEBKIT_REF"
        git -C "$WEBKIT_ROOT" checkout --detach FETCH_HEAD
    fi
fi

BUILD_SCRIPT="$WEBKIT_ROOT/Tools/Scripts/build-jsc"
[[ -x "$BUILD_SCRIPT" ]] || { echo "Not found: $BUILD_SCRIPT" >&2; exit 1; }

if [[ "$IS_DARWIN" -eq 1 ]] && ! xcodebuild -version >/dev/null 2>&1; then
    echo "Full Xcode required for build-jsc on macOS (xcodebuild unavailable)." >&2
    echo "Install Xcode.app and: sudo xcode-select -s /Applications/Xcode.app/Contents/Developer" >&2
    exit 1
fi

# --- Build ------------------------------------------------------------------
BUILD_DIR="$WEBKIT_ROOT/WebKitBuild"

# Build ICU as static archives so the artifact can ship libicu{i18n,uc,data}.a
# that build.rs links with `static=` (distro libicu-dev is shared-only). Sets
# ICU_PREFIX. Apple uses the system ICU (icucore) and skips this.
ICU_PREFIX=""
build_static_icu() {
    ICU_PREFIX="$BUILD_DIR/icu-static"
    [[ -f "$ICU_PREFIX/lib/libicuuc.a" ]] && return
    local tag="release-${ICU_VERSION//./-}"      # 74.2 -> release-74-2
    local file="icu4c-${ICU_VERSION//./_}-src.tgz" # 74.2 -> icu4c-74_2-src.tgz
    local url="https://github.com/unicode-org/icu/releases/download/${tag}/${file}"
    local src="$BUILD_DIR/icu-src"
    echo "==> Building static ICU $ICU_VERSION" >&2
    rm -rf "$src"; mkdir -p "$src"
    ( cd "$src" && curl -fSL "$url" -o icu.tgz && tar xzf icu.tgz ) >&2
    ( cd "$src/icu/source" \
        && ./configure --prefix="$ICU_PREFIX" --enable-static --disable-shared \
             --disable-tests --disable-samples \
        && make -j"${JOBS:-$(command -v nproc >/dev/null 2>&1 && nproc || echo 4)}" \
        && make install ) >&2
}

declare -a BUILD_CMD=("$BUILD_SCRIPT" "--release")
# Skip the TestWebKitAPI/TestWTF target — it's irrelevant here and trips a
# macOS "Objective-C disabled in PCH" quirk on JSCOnly.
BUILD_CMD+=("--cmakeargs=-DENABLE_API_TESTS=OFF")
[[ -n "$JOBS" ]] && BUILD_CMD+=("--makeargs=-j$JOBS")

if [[ "$IS_DARWIN" -eq 0 ]]; then
    BUILD_CMD+=("--build-dir=$BUILD_DIR")
    # Static JSC/WTF/bmalloc + static ICU, matching build.rs's linux link set
    # (static=JavaScriptCore/WTF/bmalloc + static=icui18n/icuuc/icudata).
    # VERIFY ON CI: (1) confirm the option is named ENABLE_STATIC_JSC in this
    # WebKit revision; (2) confirm WTF/bmalloc still emit as separate
    # libWTF.a/libbmalloc.a — if they fold into libJavaScriptCore.a, drop
    # static=WTF/static=bmalloc from build.rs's linux branch; (3) confirm CMake
    # FindICU picks the static ICU_ROOT over any system ICU.
    BUILD_CMD+=("--cmakeargs=-DENABLE_STATIC_JSC=ON")
    build_static_icu
    BUILD_CMD+=("--cmakeargs=-DICU_ROOT=$ICU_PREFIX")
fi

echo "==> Building: ${BUILD_CMD[*]}"
( cd "$WEBKIT_ROOT" && "${BUILD_CMD[@]}" )

# --- Locate build outputs ---------------------------------------------------
# Apple build-jsc → WebKitBuild/Release/JavaScriptCore.framework
# CMake/JSCOnly   → WebKitBuild/JSCOnly/Release/{lib,...}
FRAMEWORK=""
for c in "$BUILD_DIR/Release/JavaScriptCore.framework" \
         "$BUILD_DIR/JSCOnly/Release/JavaScriptCore.framework"; do
    [[ -d "$c" ]] && { FRAMEWORK="$c"; break; }
done

LIB_SRC=""
for c in "$BUILD_DIR/JSCOnly/Release/lib" "$BUILD_DIR/Release/lib" "$BUILD_DIR/Release"; do
    [[ -d "$c" ]] && { LIB_SRC="$c"; break; }
done

# --- Assemble the normalized install tree -----------------------------------
echo "==> Installing into $OUT_DIR"
rm -rf "$OUT_DIR"
mkdir -p "$OUT_DIR/include/JavaScriptCore" "$OUT_DIR/lib"

# WebKit's Headers/ are forwarding symlinks into the source tree; `cp -RL`
# dereferences them so the artifact survives deleting the checkout.
copy_headers() { # <src-dir> <dest-dir>
    [[ -d "$1" ]] || return 0
    mkdir -p "$2"
    cp -RL "$1"/. "$2"/ 2>/dev/null || true
}

if [[ -n "$FRAMEWORK" ]]; then
    ARTIFACT_KIND="framework"
    echo "    framework: $FRAMEWORK"
    cp -RL "$FRAMEWORK" "$OUT_DIR/lib/JavaScriptCore.framework"
    copy_headers "$FRAMEWORK/Headers"        "$OUT_DIR/include/JavaScriptCore"
    copy_headers "$FRAMEWORK/PrivateHeaders"  "$OUT_DIR/include/JavaScriptCore/private"
    # Stamp an absolute install name so dependents load it without an rpath.
    FW_BIN="$OUT_DIR/lib/JavaScriptCore.framework/Versions/A/JavaScriptCore"
    [[ -f "$FW_BIN" ]] && install_name_tool -id "$FW_BIN" "$FW_BIN" || true
elif [[ -n "$LIB_SRC" ]]; then
    ARTIFACT_KIND="static"
    echo "    static libs: $LIB_SRC"
    for a in JavaScriptCore WTF bmalloc; do
        find "$LIB_SRC" -maxdepth 1 -name "lib$a.a" -exec cp {} "$OUT_DIR/lib/" \; 2>/dev/null || true
    done
    # Public + private headers from the JSCOnly forwarding-header trees.
    # NOTE: the exact non-Apple header locations vary by WebKit revision; adjust
    # these globs if JavaScript.h / private headers are missing after a build.
    copy_headers "$BUILD_DIR/JSCOnly/Release/JavaScriptCore/Headers"        "$OUT_DIR/include/JavaScriptCore"
    copy_headers "$BUILD_DIR/JSCOnly/Release/JavaScriptCore/PrivateHeaders" "$OUT_DIR/include/JavaScriptCore/private"
    copy_headers "$BUILD_DIR/JSCOnly/Release/WTF/Headers"                   "$OUT_DIR/include/WTF"
    copy_headers "$BUILD_DIR/JSCOnly/Release/bmalloc/Headers"               "$OUT_DIR/include/bmalloc"
else
    echo "Could not locate a JavaScriptCore framework or lib dir under $BUILD_DIR" >&2
    exit 1
fi

# (non-Apple/static) bundle the static ICU archives that build.rs links with
# static=icui18n/icuuc/icudata.
if [[ "$IS_DARWIN" -eq 0 && -n "$ICU_PREFIX" ]]; then
    for a in icui18n icuuc icudata; do
        cp "$ICU_PREFIX/lib/lib$a.a" "$OUT_DIR/lib/" 2>/dev/null \
            || echo "WARNING: static ICU lib$a.a missing in $ICU_PREFIX/lib" >&2
    done
fi

require_file() {
    [[ -f "$1" ]] || { echo "Missing required file: $1" >&2; exit 1; }
}

require_dir() {
    [[ -d "$1" ]] || { echo "Missing required directory: $1" >&2; exit 1; }
}

echo "==> Validating artifact"
require_file "$OUT_DIR/include/JavaScriptCore/JavaScript.h"
require_file "$OUT_DIR/include/JavaScriptCore/private/Completion.h"
require_file "$OUT_DIR/include/JavaScriptCore/private/BytecodeCacheError.h"
require_dir "$OUT_DIR/include/WTF"
require_dir "$OUT_DIR/include/bmalloc"

if [[ "${ARTIFACT_KIND:-}" == "framework" ]]; then
    require_dir "$OUT_DIR/lib/JavaScriptCore.framework"
else
    for a in JavaScriptCore WTF bmalloc; do
        require_file "$OUT_DIR/lib/lib$a.a"
    done
    if [[ "$IS_DARWIN" -eq 0 ]]; then
        for a in icui18n icuuc icudata; do
            require_file "$OUT_DIR/lib/lib$a.a"
        done
    fi
fi

cat > "$OUT_DIR/rong-jsc-artifact.json" <<EOF
{
  "format": 1,
  "source_repo": "$WEBKIT_URL",
  "webkit_ref": "$WEBKIT_REF",
  "target": "$TARGET",
  "kind": "${ARTIFACT_KIND:-unknown}",
  "bytecode_private_headers": true
}
EOF

HOST_TARGET=""
if command -v rustc >/dev/null 2>&1; then
    HOST_TARGET="$(rustc -vV | awk '/^host:/{print $2}')"
fi
if [[ "${RONG_JSC_SKIP_SMOKE:-0}" != "1" && "$HOST_TARGET" == "$TARGET" ]]; then
    echo "==> Smoke-testing artifact"
    ( cd "$ROOT_DIR" && \
      RONG_JSC_ROOT="$OUT_DIR" \
      RONG_JSC_REQUIRE_BYTECODE=1 \
      cargo test --release --test eval --no-default-features \
        --features jscore-source,tls-aws-lc --quiet )
fi

echo "Installed artifact:"
echo "  include: $OUT_DIR/include"
echo "  lib:     $OUT_DIR/lib"

# --- Optional: package + checksum -------------------------------------------
if [[ -n "$PACKAGE" ]]; then
    echo "==> Packaging $PACKAGE"
    mkdir -p "$(dirname "$PACKAGE")"
    tar -czf "$PACKAGE" -C "$OUT_DIR" include lib
    if command -v shasum >/dev/null 2>&1; then
        SHA="$(shasum -a 256 "$PACKAGE" | awk '{print $1}')"
    else
        SHA="$(sha256sum "$PACKAGE" | awk '{print $1}')"
    fi
    echo "  tarball: $PACKAGE"
    echo "  sha256:  $SHA"
    echo
    echo "webkit-artifacts.tsv row (fill in <release-tag>):"
    echo "  $TARGET   <release-tag>   $(basename "$PACKAGE")   $SHA"
fi
