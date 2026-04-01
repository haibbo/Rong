mod compression;
mod readable;
mod writable;

pub use compression::{CompressionStream, DecompressionStream};
pub use readable::{
    JSReadableStream, ReadableStream, ReadableStreamDefaultController, ReadableStreamDefaultReader,
    readable_stream_is_locked, readable_stream_take_receiver,
};
pub use writable::{
    JSWritableStream, WritableStream, WritableStreamDefaultWriter, writable_stream_to_async_write,
    writable_stream_to_sender, writable_stream_to_sender_with_done,
};

use rong::*;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    readable::init(ctx)?;
    writable::init(ctx)?;
    compression::init(ctx)?;
    Ok(())
}
