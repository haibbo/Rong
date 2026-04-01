//! Compression utilities attached to `globalThis.Rong`.

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use rong::{
    AnyJSTypedArray, HostError, JSArrayBuffer, JSContext, JSFunc, JSObject, JSResult, JSTypedArray,
    JSValue,
};
use std::io::Cursor;
use std::io::{Read, Write};

const DEFAULT_ZSTD_LEVEL: i32 = 3;
const MIN_ZSTD_LEVEL: i32 = 1;
const MAX_ZSTD_LEVEL: i32 = 22;
const DEFAULT_GZIP_LEVEL: i32 = -1;
const MIN_GZIP_LEVEL: i32 = -1;
const MAX_GZIP_LEVEL: i32 = 9;

#[derive(Default)]
struct ZstdCompressOptions {
    level: i32,
}

#[derive(Default)]
struct GzipCompressOptions {
    level: i32,
}

fn invalid_input_type(label: &str) -> HostError {
    HostError::new(
        rong::error::E_TYPE,
        format!("{label} must be an ArrayBuffer or TypedArray"),
    )
    .with_name("TypeError")
}

fn invalid_options_type() -> HostError {
    HostError::new(
        rong::error::E_TYPE,
        "options must be an object with an optional level number",
    )
    .with_name("TypeError")
}

fn parse_input_bytes(value: &JSValue, label: &str) -> JSResult<Vec<u8>> {
    let obj = value
        .clone()
        .into_object()
        .ok_or_else(|| invalid_input_type(label))?;

    if let Some(typed_array) = AnyJSTypedArray::from_object(obj.clone()) {
        let bytes = typed_array
            .byte_view()
            .ok_or_else(|| invalid_input_type(label))?;
        return Ok(bytes.to_vec());
    }

    if let Some(buffer) = JSArrayBuffer::from_object(obj) {
        return Ok(buffer.to_vec());
    }

    Err(invalid_input_type(label).into())
}

fn parse_compress_options(options: Option<JSObject>) -> JSResult<ZstdCompressOptions> {
    let Some(options) = options else {
        return Ok(ZstdCompressOptions {
            level: DEFAULT_ZSTD_LEVEL,
        });
    };

    if options.has_property("level")? {
        let level = options
            .get::<_, f64>("level")
            .map_err(|_| invalid_options_type())?;
        if !level.is_finite() || level.fract() != 0.0 {
            return Err(HostError::new(
                rong::error::E_TYPE,
                "options.level must be an integer between 1 and 22",
            )
            .with_name("TypeError")
            .into());
        }

        let level = level as i32;
        if !(MIN_ZSTD_LEVEL..=MAX_ZSTD_LEVEL).contains(&level) {
            return Err(HostError::new(
                rong::error::E_TYPE,
                "options.level must be an integer between 1 and 22",
            )
            .with_name("TypeError")
            .into());
        }

        return Ok(ZstdCompressOptions { level });
    }

    Ok(ZstdCompressOptions {
        level: DEFAULT_ZSTD_LEVEL,
    })
}

fn parse_gzip_options(options: Option<JSObject>) -> JSResult<GzipCompressOptions> {
    let Some(options) = options else {
        return Ok(GzipCompressOptions {
            level: DEFAULT_GZIP_LEVEL,
        });
    };

    if options.has_property("level")? {
        let level = options
            .get::<_, f64>("level")
            .map_err(|_| invalid_options_type())?;
        if !level.is_finite() || level.fract() != 0.0 {
            return Err(HostError::new(
                rong::error::E_TYPE,
                "options.level must be an integer between -1 and 9",
            )
            .with_name("TypeError")
            .into());
        }

        let level = level as i32;
        if !(MIN_GZIP_LEVEL..=MAX_GZIP_LEVEL).contains(&level) {
            return Err(HostError::new(
                rong::error::E_TYPE,
                "options.level must be an integer between -1 and 9",
            )
            .with_name("TypeError")
            .into());
        }

        return Ok(GzipCompressOptions { level });
    }

    Ok(GzipCompressOptions {
        level: DEFAULT_GZIP_LEVEL,
    })
}

fn bytes_to_uint8_array(ctx: &JSContext, bytes: Vec<u8>) -> JSResult<JSTypedArray<u8>> {
    let len = bytes.len();
    let buffer = JSArrayBuffer::from_bytes_owned(ctx, bytes)?;
    JSTypedArray::<u8>::from_array_buffer(ctx, buffer, 0, Some(len))
}

fn compress_zstd_sync(
    ctx: JSContext,
    input: JSValue,
    options: rong::function::Optional<JSObject>,
) -> JSResult<JSTypedArray<u8>> {
    let input = parse_input_bytes(&input, "data")?;
    let options = parse_compress_options(options.0)?;
    let compressed =
        zstd::stream::encode_all(Cursor::new(input), options.level).map_err(|error| {
            HostError::new(
                rong::error::E_IO,
                format!("zstd compression failed: {error}"),
            )
        })?;
    bytes_to_uint8_array(&ctx, compressed)
}

fn gzip_sync(
    ctx: JSContext,
    input: JSValue,
    options: rong::function::Optional<JSObject>,
) -> JSResult<JSTypedArray<u8>> {
    let input = parse_input_bytes(&input, "data")?;
    let options = parse_gzip_options(options.0)?;
    let compression = if options.level == -1 {
        Compression::default()
    } else {
        Compression::new(options.level as u32)
    };

    let mut encoder = GzEncoder::new(Vec::new(), compression);
    encoder.write_all(&input).map_err(|error| {
        HostError::new(
            rong::error::E_IO,
            format!("gzip compression failed: {error}"),
        )
    })?;
    let compressed = encoder.finish().map_err(|error| {
        HostError::new(
            rong::error::E_IO,
            format!("gzip compression failed: {error}"),
        )
    })?;
    bytes_to_uint8_array(&ctx, compressed)
}

async fn gzip(
    ctx: JSContext,
    input: JSValue,
    options: rong::function::Optional<JSObject>,
) -> JSResult<JSTypedArray<u8>> {
    let input = parse_input_bytes(&input, "data")?;
    let options = parse_gzip_options(options.0)?;
    let compressed = tokio::task::spawn_blocking(move || {
        let compression = if options.level == -1 {
            Compression::default()
        } else {
            Compression::new(options.level as u32)
        };

        let mut encoder = GzEncoder::new(Vec::new(), compression);
        encoder.write_all(&input)?;
        encoder.finish()
    })
    .await
    .map_err(|error| {
        HostError::new(
            rong::error::E_INTERNAL,
            format!("gzip compression task failed: {error}"),
        )
    })?
    .map_err(|error| {
        HostError::new(
            rong::error::E_IO,
            format!("gzip compression failed: {error}"),
        )
    })?;
    bytes_to_uint8_array(&ctx, compressed)
}

fn decompress_zstd_sync(ctx: JSContext, input: JSValue) -> JSResult<JSTypedArray<u8>> {
    let input = parse_input_bytes(&input, "data")?;
    let decompressed = zstd::stream::decode_all(Cursor::new(input)).map_err(|error| {
        HostError::new(
            rong::error::E_IO,
            format!("zstd decompression failed: {error}"),
        )
    })?;
    bytes_to_uint8_array(&ctx, decompressed)
}

fn gunzip_sync(ctx: JSContext, input: JSValue) -> JSResult<JSTypedArray<u8>> {
    let input = parse_input_bytes(&input, "data")?;
    let mut decoder = GzDecoder::new(Cursor::new(input));
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|error| {
        HostError::new(
            rong::error::E_IO,
            format!("gzip decompression failed: {error}"),
        )
    })?;
    bytes_to_uint8_array(&ctx, decompressed)
}

async fn gunzip(ctx: JSContext, input: JSValue) -> JSResult<JSTypedArray<u8>> {
    let input = parse_input_bytes(&input, "data")?;
    let decompressed = tokio::task::spawn_blocking(move || {
        let mut decoder = GzDecoder::new(Cursor::new(input));
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok::<Vec<u8>, std::io::Error>(decompressed)
    })
    .await
    .map_err(|error| {
        HostError::new(
            rong::error::E_INTERNAL,
            format!("gzip decompression task failed: {error}"),
        )
    })?
    .map_err(|error| {
        HostError::new(
            rong::error::E_IO,
            format!("gzip decompression failed: {error}"),
        )
    })?;
    bytes_to_uint8_array(&ctx, decompressed)
}

async fn compress_zstd(
    ctx: JSContext,
    input: JSValue,
    options: rong::function::Optional<JSObject>,
) -> JSResult<JSTypedArray<u8>> {
    let input = parse_input_bytes(&input, "data")?;
    let options = parse_compress_options(options.0)?;
    let compressed = tokio::task::spawn_blocking(move || {
        zstd::stream::encode_all(Cursor::new(input), options.level)
    })
    .await
    .map_err(|error| {
        HostError::new(
            rong::error::E_INTERNAL,
            format!("zstd compression task failed: {error}"),
        )
    })?
    .map_err(|error| {
        HostError::new(
            rong::error::E_IO,
            format!("zstd compression failed: {error}"),
        )
    })?;
    bytes_to_uint8_array(&ctx, compressed)
}

async fn decompress_zstd(ctx: JSContext, input: JSValue) -> JSResult<JSTypedArray<u8>> {
    let input = parse_input_bytes(&input, "data")?;
    let decompressed =
        tokio::task::spawn_blocking(move || zstd::stream::decode_all(Cursor::new(input)))
            .await
            .map_err(|error| {
                HostError::new(
                    rong::error::E_INTERNAL,
                    format!("zstd decompression task failed: {error}"),
                )
            })?
            .map_err(|error| {
                HostError::new(
                    rong::error::E_IO,
                    format!("zstd decompression failed: {error}"),
                )
            })?;
    bytes_to_uint8_array(&ctx, decompressed)
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.host_namespace();

    let zstd_compress = JSFunc::new(ctx, compress_zstd)?.name("zstdCompress")?;
    rong.set("zstdCompress", zstd_compress)?;

    let zstd_compress_sync = JSFunc::new(ctx, compress_zstd_sync)?.name("zstdCompressSync")?;
    rong.set("zstdCompressSync", zstd_compress_sync)?;

    let zstd_decompress = JSFunc::new(ctx, decompress_zstd)?.name("zstdDecompress")?;
    rong.set("zstdDecompress", zstd_decompress)?;

    let zstd_decompress_sync =
        JSFunc::new(ctx, decompress_zstd_sync)?.name("zstdDecompressSync")?;
    rong.set("zstdDecompressSync", zstd_decompress_sync)?;

    let gzip_fn = JSFunc::new(ctx, gzip)?.name("gzip")?;
    rong.set("gzip", gzip_fn)?;

    let gzip_sync_fn = JSFunc::new(ctx, gzip_sync)?.name("gzipSync")?;
    rong.set("gzipSync", gzip_sync_fn)?;

    let gunzip_fn = JSFunc::new(ctx, gunzip)?.name("gunzip")?;
    rong.set("gunzip", gunzip_fn)?;

    let gunzip_sync_fn = JSFunc::new(ctx, gunzip_sync)?.name("gunzipSync")?;
    rong.set("gunzipSync", gunzip_sync_fn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    #[test]
    fn test_compression_namespace() {
        async_run!(|ctx: JSContext| async move {
            rong_assert::init(&ctx)?;
            rong_console::init(&ctx)?;
            rong_encoding::init(&ctx)?;
            init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "compression.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
