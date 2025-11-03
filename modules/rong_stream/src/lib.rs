mod readable;
mod writable;

pub use readable::{
    JSReadableStream, ReadableStream, ReadableStreamDefaultController, ReadableStreamDefaultReader,
    readable_stream_is_locked, readable_stream_take_receiver,
};
pub use writable::{
    WritableStream, WritableStreamDefaultWriter, writable_stream_to_async_write,
    writable_stream_to_sender,
};

use rong::*;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    readable::init(ctx)?;
    writable::init(ctx)?;
    Ok(())
}
