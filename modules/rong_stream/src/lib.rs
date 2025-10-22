pub mod readable;
pub mod writable;

pub use readable::{
    ReadableStream, ReadableStreamDefaultController, ReadableStreamDefaultReader,
    readable_stream_from_async_read, readable_stream_from_receiver, readable_stream_take_receiver,
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
