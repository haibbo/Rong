use crate::grant_file_access;
use crate::stat::FileInfo;
use bytes::Bytes;
use rong::{function::Optional, *};
use rong_stream::{JSReadableStream, WritableStream};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::sync::Mutex;
use tokio::sync::mpsc;

#[derive(FromJSObj)]
struct FileOpenOption {
    read: Option<bool>,
    write: Option<bool>,
    append: Option<bool>,
    truncate: Option<bool>,
    create: Option<bool>,
    #[rename = "createNew"]
    create_new: Option<bool>,
    mode: Option<u32>,
}

#[js_export]
struct FsFile {
    file: Arc<Mutex<File>>,
}

#[js_class]
impl FsFile {
    #[js_method(constructor)]
    fn new() -> JSResult<Self> {
        Err(RongJSError::TypeError(
            "Not Allowed 'new FsFile()', use Rong.open".to_string(),
        ))
    }

    #[js_method]
    async fn stat(&self) -> JSResult<FileInfo> {
        let file = self.file.lock().await;
        file.metadata()
            .await
            .map(FileInfo::from_metadata)
            .map_err(|e| RongJSError::TypeError(format!("Failed to get file stats: {}", e)))
    }

    #[js_method]
    async fn read(&mut self, buffer: JSArrayBuffer<u8>) -> JSResult<Option<usize>> {
        // Get buffer length
        let buf_len = buffer.len();
        if buf_len == 0 {
            return Ok(Some(0));
        }

        // Get direct mutable access to the ArrayBuffer's data
        let mut buffer = buffer;
        let buffer_slice = buffer.as_mut_slice();

        // Read directly into the ArrayBuffer
        let mut file = self.file.lock().await;
        match file.read(buffer_slice).await {
            Ok(0) => Ok(None), // EOF - return null like Deno
            Ok(bytes_read) => Ok(Some(bytes_read)),
            Err(e) => Err(RongJSError::TypeError(format!(
                "Failed to read file: {}",
                e
            ))),
        }
    }

    #[js_method]
    async fn write(&mut self, buffer: JSArrayBuffer<u8>) -> JSResult<usize> {
        // Get buffer data
        let buf = buffer.as_slice();

        // Write to file
        let mut file = self.file.lock().await;
        file.write_all(buf)
            .await
            .map_err(|e| RongJSError::TypeError(format!("Failed to write file: {}", e)))?;

        Ok(buf.len())
    }

    #[js_method]
    async fn sync(&mut self) -> JSResult<()> {
        let file = self.file.lock().await;
        file.sync_all()
            .await
            .map_err(|e| RongJSError::TypeError(format!("Failed to sync file: {}", e)))
    }

    #[js_method]
    async fn truncate(&mut self, len: Optional<u64>) -> JSResult<()> {
        let length = len.0.unwrap_or(0);
        let file = self.file.lock().await;
        file.set_len(length)
            .await
            .map_err(|e| RongJSError::TypeError(format!("Failed to truncate file: {}", e)))
    }

    #[js_method]
    async fn seek(&mut self, offset: i64, whence: Optional<u32>) -> JSResult<u64> {
        let whence_mode = whence.0.unwrap_or(0); // Default to Start (0)

        // Convert whence number to SeekFrom
        let seek_from = match whence_mode {
            0 => SeekFrom::Start(offset as u64), // Rong.SeekMode.Start
            1 => SeekFrom::Current(offset),      // Rong.SeekMode.Current
            2 => SeekFrom::End(offset),          // Rong.SeekMode.End
            _ => {
                return Err(RongJSError::TypeError(format!(
                    "Invalid whence value: {}. Must be 0 (Start), 1 (Current), or 2 (End)",
                    whence_mode
                )));
            }
        };

        // Perform the seek operation
        let mut file = self.file.lock().await;
        let new_position = file
            .seek(seek_from)
            .await
            .map_err(|e| RongJSError::TypeError(format!("Failed to seek: {}", e)))?;

        Ok(new_position)
    }

    #[js_method]
    async fn close(&mut self) -> JSResult<()> {
        // Sync before closing
        self.sync().await
    }

    #[js_method(getter)]
    fn readable(&self, ctx: JSContext) -> Option<JSObject> {
        // Create a channel-backed ReadableStream that reads from this file
        let file = self.file.clone();
        let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(16);
        let chunk_size = 64 * 1024; // 64 KiB default chunk size (similar to Deno defaults)
        tokio::task::spawn(async move {
            let mut buf = vec![0u8; chunk_size];
            let mut f = file.lock().await;
            loop {
                match f.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx
                            .send(Ok(Bytes::copy_from_slice(&buf[..n])))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string())).await;
                        break;
                    }
                }
            }
        });
        // from_receiver should not fail after rong_stream::init; fallback to empty object on error
        JSReadableStream::from_receiver(&ctx, rx)
            .map(|jsrs| jsrs.into_object())
            .ok()
    }

    #[js_method(getter)]
    fn writable(&self) -> JSResult<WritableStream> {
        // Create a channel-backed WritableStream that writes to this file
        let file = self.file.clone();
        // Larger channel buffers smooth out bursts from network streams
        // and reduce backpressure on JS when writing large files.
        let (tx, mut rx) = mpsc::channel::<Bytes>(128);
        tokio::task::spawn(async move {
            let mut f = file.lock().await;
            while let Some(chunk) = rx.recv().await {
                if let Err(_e) = f.write_all(&chunk).await {
                    break;
                }
            }
            let _ = f.flush().await;
        });
        Ok(rong_stream::writable_stream_to_sender(tx))
    }
}

async fn open_file(file: String, option: Optional<FileOpenOption>) -> JSResult<FsFile> {
    let resolved = grant_file_access(&file)?;

    let opts = option.0.unwrap_or(FileOpenOption {
        read: None,
        write: None,
        append: None,
        truncate: None,
        create: None,
        create_new: None,
        mode: None,
    });
    // Apply Deno defaults
    let read = opts.read.unwrap_or(true);
    let write = opts.write.unwrap_or(false);
    let append = opts.append.unwrap_or(false);
    let truncate = opts.truncate.unwrap_or(false);
    let create = opts.create.unwrap_or(false);
    let create_new = opts.create_new.unwrap_or(false);
    let _mode = opts.mode; // TODO: implement mode support

    // Configure file open options
    let mut open_options = OpenOptions::new();
    open_options
        .read(read)
        .write(write)
        .append(append)
        .truncate(truncate)
        .create(create)
        .create_new(create_new);

    // Open the file
    let file_handle = open_options
        .open(&resolved)
        .await
        .map_err(|e| RongJSError::TypeError(format!("Failed to open file '{}': {}", file, e)))?;

    Ok(FsFile {
        file: Arc::new(Mutex::new(file_handle)),
    })
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.rong();

    ctx.register_class::<FsFile>()?;

    let open_fn = JSFunc::new(ctx, open_file)?.name("open")?;
    rong.set("open", open_fn)?;

    let seek_mode = JSObject::new(ctx);
    seek_mode.set("Start", 0u32)?;
    seek_mode.set("Current", 1u32)?;
    seek_mode.set("End", 2u32)?;
    rong.set("SeekMode", seek_mode)?;

    Ok(())
}
