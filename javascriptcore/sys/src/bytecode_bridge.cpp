/// JavaScriptCore bytecode bridge — compiled as C++ with access to JSC private
/// headers. Exposes `extern "C"` entry points for the Rust layer:
///
///   rong_jsc_compile_to_bytecode  →  source code → malloc'd bytecode buffer
///   rong_jsc_run_bytecode         →  bytecode buffer → JSValueRef result
///   rong_jsc_free_bytecode        →  free a buffer from compile_to_bytecode
///   rong_jsc_free_error           →  free an owned bridge error string
///
/// The bridge is always compiled and linked for the source/JSCOnly backend
/// (`#[cfg(jsc_source)]` on the Rust side), so these symbols are always defined.
/// `build.rs` defines `RONG_JSC_HAVE_PRIVATE_HEADERS` only when the JSC artifact
/// ships the private/internal headers the real implementation needs; otherwise
/// the stub at the bottom of this file is compiled and bytecode is reported as
/// unsupported at runtime.

#include <JavaScriptCore/JavaScript.h>

#include <cstddef>
#include <cstdint>
#include <utility>

#if __has_include(<expected>)
#include <expected>
#endif

// Current WebKit WTF exposes its Expected alias through a Library Fundamentals
// fallback, but the bundled Expected.h references unexpected<E> without defining
// it when used from installed private headers. C++23 standard libraries provide
// std::unexpected, which unqualified lookup can find from std::experimental; use
// this shim only for older standard libraries.
#if !defined(__cpp_lib_expected)
namespace std {
namespace experimental {
inline namespace fundamentals_v3 {
template<class E> class unexpected {
public:
    constexpr explicit unexpected(const E& error)
        : m_error(error)
    {
    }

    constexpr explicit unexpected(E&& error)
        : m_error(std::move(error))
    {
    }

    constexpr E& error() &
    {
        return m_error;
    }

    constexpr const E& error() const&
    {
        return m_error;
    }

    constexpr E&& error() &&
    {
        return std::move(m_error);
    }

    constexpr const E&& error() const&&
    {
        return std::move(m_error);
    }

private:
    E m_error;
};
}
}

template<class E> using unexpected = experimental::unexpected<E>;
}
#endif

#if __has_include("InitializeThreading.h")
#include "InitializeThreading.h"
#define RONG_JSC_CAN_INITIALIZE 1
#elif __has_include(<JavaScriptCore/InitializeThreading.h>)
#include <JavaScriptCore/InitializeThreading.h>
#define RONG_JSC_CAN_INITIALIZE 1
#endif

// The result struct is part of the ABI shared with the Rust FFI declarations,
// so it is defined for both the real and stub builds.
extern "C" {
typedef struct {
    uint8_t*    data;   // malloc'd buffer, caller frees with rong_jsc_free_bytecode
    size_t      size;   // buffer size in bytes
    const char* error;  // NULL on success; malloc'd error message on failure
} RongJSCBytecodeResult;

typedef struct {
    JSValueRef  value;        // JS result or thrown JS value
    int         is_exception; // non-zero when value must be treated as thrown
    const char* error;        // NULL normally; malloc'd bridge/internal error
} RongJSCRunBytecodeResult;
}

extern "C" void rong_jsc_initialize(void)
{
#if defined(RONG_JSC_CAN_INITIALIZE)
    JSC::initialize();
#endif
}

#if defined(RONG_JSC_HAVE_PRIVATE_HEADERS)

// ---------------------------------------------------------------------------
// JSC private / internal headers — these live under
// <cache>/include/JavaScriptCore/private/ in bytecode-capable artifacts.
// ---------------------------------------------------------------------------

// Prefer the canonical <JavaScriptCore/...> private-header path when source
// artifacts expose it. Falling back to flat private headers keeps older local
// artifacts usable, but mixing both paths can bypass #pragma once.
#if __has_include("config.h")
#include "config.h"
#endif

#ifndef JS_EXPORT_PRIVATE
#define JS_EXPORT_PRIVATE
#endif
#ifndef WTF_EXPORT_PRIVATE
#define WTF_EXPORT_PRIVATE
#endif

#if __has_include(<JavaScriptCore/APICast.h>)
#include <JavaScriptCore/APICast.h>
#include <JavaScriptCore/BytecodeCacheError.h>
#include <JavaScriptCore/CachedBytecode.h>
#include <JavaScriptCore/Completion.h>
#include <JavaScriptCore/Exception.h>
#include <JavaScriptCore/SourceCode.h>
#include <JavaScriptCore/SourceProvider.h>
#include <JavaScriptCore/SourceTaintedOrigin.h>
#include <JavaScriptCore/ThrowScope.h>
#include <JavaScriptCore/VM.h>
#include <JavaScriptCore/JSCInlines.h>
#else
#include "APICast.h"
#include "BytecodeCacheError.h"
#include "CachedBytecode.h"
#include "Completion.h"
#include "Exception.h"
#include "SourceCode.h"
#include "SourceProvider.h"
#include "SourceTaintedOrigin.h"
#include "ThrowScope.h"
#include "VM.h"
#include "JSCInlines.h"
#endif

#include <cstring>
#include <limits>
#include <span>
#include <wtf/FileHandle.h>
#include <wtf/MallocSpan.h>
#include <wtf/StdLibExtras.h>
#include <wtf/text/CString.h>

// ============================================================================
// Public C interface (real implementation)
// ============================================================================

namespace {

constexpr uint8_t kMagic[] = { 'R', 'J', 'S', 'C', 'B', 'C', '1', 0 };
constexpr uint32_t kEnvelopeVersion = 1;
constexpr size_t kMagicSize = sizeof(kMagic);
constexpr size_t kVersionOffset = kMagicSize;
constexpr size_t kSourceLenOffset = kVersionOffset + sizeof(uint32_t);
constexpr size_t kURLLenOffset = kSourceLenOffset + sizeof(uint64_t);
constexpr size_t kHeaderSize = kURLLenOffset + sizeof(uint64_t);

static_assert(kMagicSize == 8);

template<typename StringType = String>
String stringFromUTF8(const char* data, size_t len)
    requires requires { StringType::fromUTF8(std::span<const char>(data, len)); }
{
    return StringType::fromUTF8(std::span<const char>(data, len));
}

template<typename StringType = String>
String stringFromUTF8(const char* data, size_t len)
    requires (!requires { StringType::fromUTF8(std::span<const char>(data, len)); })
{
    return StringType::fromUTF8(data, len);
}

char* copyCString(const char* message)
{
    if (!message)
        message = "unknown JavaScriptCore bytecode bridge error";
    size_t len = std::strlen(message);
    char* out = static_cast<char*>(fastMalloc(len + 1));
    std::memcpy(out, message, len + 1);
    return out;
}

char* copyString(const String& message)
{
    auto utf8 = message.utf8();
    return copyCString(utf8.data());
}

RongJSCBytecodeResult compileError(const char* message)
{
    return { nullptr, 0, copyCString(message) };
}

RongJSCBytecodeResult compileError(const String& message)
{
    return { nullptr, 0, copyString(message) };
}

RongJSCRunBytecodeResult runError(const char* message)
{
    return { nullptr, 0, copyCString(message) };
}

RongJSCRunBytecodeResult runException(JSValueRef value)
{
    return { value, 1, nullptr };
}

RongJSCRunBytecodeResult runValue(JSValueRef value)
{
    return { value, 0, nullptr };
}

void writeU32LE(uint8_t* dest, uint32_t value)
{
    dest[0] = static_cast<uint8_t>(value);
    dest[1] = static_cast<uint8_t>(value >> 8);
    dest[2] = static_cast<uint8_t>(value >> 16);
    dest[3] = static_cast<uint8_t>(value >> 24);
}

void writeU64LE(uint8_t* dest, uint64_t value)
{
    for (unsigned i = 0; i < 8; ++i)
        dest[i] = static_cast<uint8_t>(value >> (i * 8));
}

uint32_t readU32LE(const uint8_t* bytes)
{
    return static_cast<uint32_t>(bytes[0])
        | (static_cast<uint32_t>(bytes[1]) << 8)
        | (static_cast<uint32_t>(bytes[2]) << 16)
        | (static_cast<uint32_t>(bytes[3]) << 24);
}

uint64_t readU64LE(const uint8_t* bytes)
{
    uint64_t value = 0;
    for (unsigned i = 0; i < 8; ++i)
        value |= static_cast<uint64_t>(bytes[i]) << (i * 8);
    return value;
}

JSValueRef makeErrorValue(JSContextRef ctx, const char* message)
{
    JSStringRef messageString = JSStringCreateWithUTF8CString(message);
    JSValueRef args[] = { JSValueMakeString(ctx, messageString) };
    JSValueRef exception = nullptr;
    JSObjectRef error = JSObjectMakeError(ctx, 1, args, &exception);
    JSStringRelease(messageString);
    if (exception)
        return exception;
    return error;
}

JSC::SourceCode makeProgramSource(const String& source, const String& sourceURL)
{
    JSC::SourceOrigin origin { URL({ }, sourceURL) };
    return JSC::makeSource(
        source,
        origin,
        JSC::SourceTaintedOrigin::Untainted,
        sourceURL,
        TextPosition(),
        JSC::SourceProviderSourceType::Program);
}

class RongCachedSourceProvider final : public JSC::SourceProvider {
public:
    static Ref<RongCachedSourceProvider> create(
        String source,
        const JSC::SourceOrigin& sourceOrigin,
        String sourceURL,
        RefPtr<JSC::CachedBytecode>&& bytecode)
    {
        return adoptRef(*new RongCachedSourceProvider(
            WTF::move(source),
            sourceOrigin,
            WTF::move(sourceURL),
            WTF::move(bytecode)));
    }

    unsigned hash() const override
    {
        return m_source.hash();
    }

    StringView source() const override
    {
        return m_source;
    }

    RefPtr<JSC::CachedBytecode> cachedBytecode() const override
    {
        return m_bytecode;
    }

private:
    RongCachedSourceProvider(
        String&& source,
        const JSC::SourceOrigin& sourceOrigin,
        String&& sourceURL,
        RefPtr<JSC::CachedBytecode>&& bytecode)
        : JSC::SourceProvider(
            sourceOrigin,
            WTF::move(sourceURL),
            String(),
            JSC::SourceTaintedOrigin::Untainted,
            TextPosition(),
            JSC::SourceProviderSourceType::Program)
        , m_source(WTF::move(source))
        , m_bytecode(WTF::move(bytecode))
    {
    }

    String m_source;
    RefPtr<JSC::CachedBytecode> m_bytecode;
};

} // namespace

extern "C" {

/// Whether the real bytecode implementation is available (1) or this is the
/// stub (0). Lets the Rust layer report a clean "not supported" error for
/// framework-only artifacts without a second build cfg.
int rong_jsc_bytecode_supported(void) {
    return 1;
}

/// Free a bytecode buffer returned by rong_jsc_compile_to_bytecode.
void rong_jsc_free_bytecode(uint8_t* data) {
    if (data) {
        fastFree(data);
    }
}

void rong_jsc_free_error(const char* error) {
    if (error) {
        fastFree(const_cast<char*>(error));
    }
}

/// Compile JavaScript source code to a portable bytecode buffer.
///
/// The resulting buffer is a Rong-owned envelope containing the source bytes
/// needed for JSC cache-key validation followed by the serialized bytecode
/// payload. The caller takes ownership and must free it with
/// rong_jsc_free_bytecode().
///
/// Returns {NULL, 0, "error message"} on failure.
RongJSCBytecodeResult rong_jsc_compile_to_bytecode(
    JSContextRef ctx,
    const char*   source,
    size_t        source_len,
    const char*   source_url)
{
    using namespace JSC;

    if (!ctx)
        return compileError("invalid JavaScriptCore context");
    if (!source && source_len)
        return compileError("invalid JavaScript source pointer");
    if (!source_url)
        return compileError("invalid JavaScript source URL");

    // Recover the C++ VM object from the opaque C context handle.
    JSGlobalObject* globalObject = toJS(ctx);
    VM& vm = globalObject->vm();

    JSLockHolder lock(vm);
    auto scope = DECLARE_THROW_SCOPE(vm);

    // ---------------------------------------------------------------
    // 1. Build a JSC SourceCode from the caller-supplied bytes + URL.
    // ---------------------------------------------------------------
    String sourceString    = stringFromUTF8(source, source_len);
    String urlString       = String::fromUTF8(source_url);
    SourceCode sourceCode  = makeProgramSource(sourceString, urlString);

    // ---------------------------------------------------------------
    // 2. Compile only. `generateProgramBytecode` parses and serializes the
    //    unlinked program code block without evaluating the script.
    // ---------------------------------------------------------------
    BytecodeCacheError error;
    FileSystem::FileHandle bytecodeFile;
    RefPtr<CachedBytecode> bytecode = generateProgramBytecode(
        vm,
        sourceCode,
        bytecodeFile,
        error);
    if (auto* exception = scope.exception()) {
        auto message = exception->value().toWTFString(globalObject);
        (void)scope.tryClearException();
        return compileError(message);
    }
    if (!bytecode) {
        if (error.isValid())
            return compileError(error.message());
        return compileError("JavaScriptCore failed to compile source to bytecode");
    }

    auto payload = bytecode->span();
    const uint8_t* payloadData = payload.data();
    size_t payloadSize = payload.size();
    if (!payloadData || payloadSize == 0) {
        return compileError("JavaScriptCore bytecode compilation produced an empty payload");
    }

    // ---------------------------------------------------------------
    // 3. Wrap JSC's raw bytecode with a Rong-owned envelope. JSC validates
    //    bytecode against a SourceCodeKey, so the envelope includes the source
    //    text used to generate the key. Compilation still has no side effects;
    //    execution uses this source only to select the cached bytecode.
    // ---------------------------------------------------------------
    size_t urlLen = std::strlen(source_url);
    uint64_t sourceSize = static_cast<uint64_t>(source_len);
    uint64_t urlSize = static_cast<uint64_t>(urlLen);
    if (source_len > std::numeric_limits<size_t>::max() - kHeaderSize
        || urlLen > std::numeric_limits<size_t>::max() - kHeaderSize - source_len
        || payloadSize > std::numeric_limits<size_t>::max() - kHeaderSize - source_len - urlLen)
        return compileError("JavaScriptCore bytecode envelope is too large");

    size_t totalSize = kHeaderSize + source_len + urlLen + payloadSize;
    uint8_t* buffer = static_cast<uint8_t*>(fastMalloc(totalSize));
    if (!buffer) {
        return compileError("allocation failed");
    }
    std::memcpy(buffer, kMagic, kMagicSize);
    writeU32LE(buffer + kVersionOffset, kEnvelopeVersion);
    writeU64LE(buffer + kSourceLenOffset, sourceSize);
    writeU64LE(buffer + kURLLenOffset, urlSize);
    std::memcpy(buffer + kHeaderSize, source, source_len);
    std::memcpy(buffer + kHeaderSize + source_len, source_url, urlLen);
    std::memcpy(buffer + kHeaderSize + source_len + urlLen, payloadData, payloadSize);

    return { buffer, totalSize, nullptr };
}

/// Execute previously compiled bytecode.
///
/// The bytecode buffer must have been produced by rong_jsc_compile_to_bytecode.
/// The envelope contains a format header, the original source bytes, and the
/// serialized bytecode payload.
///
/// Returns the JS result value on success. On error, returns either a thrown JS
/// value (`is_exception != 0`) or an owned bridge error string.
RongJSCRunBytecodeResult rong_jsc_run_bytecode(
    JSContextRef ctx,
    const uint8_t* bytes,
    size_t         size)
{
    using namespace JSC;

    if (!ctx)
        return runError("invalid JavaScriptCore context");
    if (!bytes && size)
        return runError("invalid JavaScriptCore bytecode pointer");

    JSGlobalObject* globalObject = toJS(ctx);
    VM& vm = globalObject->vm();

    // Hold the VM lock before touching the heap. createTypeError below (and the
    // evaluate path further down) allocate, so the lock must already be held —
    // acquiring it only after the version check would create the error value
    // without the lock.
    JSLockHolder lock(vm);
    auto scope = DECLARE_THROW_SCOPE(vm);

    if (size < kHeaderSize || std::memcmp(bytes, kMagic, kMagicSize) != 0)
        return runException(makeErrorValue(ctx, "Invalid JavaScriptCore bytecode envelope"));

    uint32_t storedVersion = readU32LE(bytes + kVersionOffset);
    if (storedVersion != kEnvelopeVersion) {
        return runException(makeErrorValue(ctx, "Unsupported JavaScriptCore bytecode envelope version"));
    }

    uint64_t sourceSize64 = readU64LE(bytes + kSourceLenOffset);
    if (sourceSize64 > static_cast<uint64_t>(size - kHeaderSize))
        return runException(makeErrorValue(ctx, "Invalid JavaScriptCore bytecode source length"));
    size_t sourceSize = static_cast<size_t>(sourceSize64);
    uint64_t urlSize64 = readU64LE(bytes + kURLLenOffset);
    if (urlSize64 > static_cast<uint64_t>(size - kHeaderSize - sourceSize))
        return runException(makeErrorValue(ctx, "Invalid JavaScriptCore bytecode URL length"));
    size_t urlSize = static_cast<size_t>(urlSize64);
    size_t payloadSize = size - kHeaderSize - sourceSize - urlSize;
    if (!payloadSize)
        return runException(makeErrorValue(ctx, "Invalid empty JavaScriptCore bytecode payload"));

    const uint8_t* sourceStart = bytes + kHeaderSize;
    const uint8_t* urlStart = sourceStart + sourceSize;
    const uint8_t* payloadStart = urlStart + urlSize;

    // We must copy the bytes into a JSC-owned allocation because
    // CachedBytecode::create takes ownership.
    auto payloadCopy = MallocSpan<uint8_t, VMMalloc>::malloc(payloadSize);
    if (!payloadCopy) {
        return runException(makeErrorValue(ctx, "JavaScriptCore bytecode allocation failed"));
    }
    std::memcpy(payloadCopy.mutableSpan().data(), payloadStart, payloadSize);

    RefPtr<CachedBytecode> cachedBytecode =
        CachedBytecode::create(WTF::move(payloadCopy), { });

    String urlString = stringFromUTF8(reinterpret_cast<const char*>(urlStart), urlSize);
    SourceOrigin origin { URL({ }, urlString) };
    String sourceString = stringFromUTF8(reinterpret_cast<const char*>(sourceStart), sourceSize);
    SourceCode sourceCode(RongCachedSourceProvider::create(
        sourceString,
        origin,
        urlString,
        WTF::move(cachedBytecode)));

    // ---------------------------------------------------------------
    // Evaluate the source with a provider that supplies cached bytecode. JSC
    // validates the SourceCodeKey and decodes the payload instead of parsing.
    // ---------------------------------------------------------------
    NakedPtr<Exception> returnedException;
    JSValue result = JSC::evaluate(globalObject, sourceCode, JSValue(), returnedException);

    if (returnedException)
        return runException(toRef(globalObject, returnedException->value()));
    if (auto* exception = scope.exception()) {
        JSValueRef exceptionValue = toRef(globalObject, exception->value());
        (void)scope.tryClearException();
        return runException(exceptionValue);
    }

    return runValue(toRef(globalObject, result));
}

} // extern "C"

#else  // !RONG_JSC_HAVE_PRIVATE_HEADERS

// ============================================================================
// Stub implementation — compiled when the JSC artifact does not ship the
// private/internal headers the real bridge needs. The symbols still link (so a
// framework-only / older artifact builds), but bytecode is reported as
// unsupported at runtime. Uses only the public JavaScriptCore C API.
// ============================================================================

extern "C" {

int rong_jsc_bytecode_supported(void) {
    return 0;
}

void rong_jsc_free_bytecode(uint8_t* /*data*/) {
    // The stub compile path never hands out a buffer, so there is nothing to free.
}

void rong_jsc_free_error(const char* /*error*/) {
    // Stub errors are static string literals.
}

RongJSCBytecodeResult rong_jsc_compile_to_bytecode(
    JSContextRef /*ctx*/,
    const char*  /*source*/,
    size_t       /*source_len*/,
    const char*  /*source_url*/)
{
    return { nullptr, 0,
             "bytecode is unsupported: JSC artifact built without private headers" };
}

RongJSCRunBytecodeResult rong_jsc_run_bytecode(
    JSContextRef /*ctx*/,
    const uint8_t* /*bytes*/,
    size_t         /*size*/)
{
    // No bytecode can have been produced by the stub compile path, so this is
    // effectively unreachable. Return NULL; the Rust layer maps that to an error.
    return { nullptr, 0, nullptr };
}

} // extern "C"

#endif // RONG_JSC_HAVE_PRIVATE_HEADERS
