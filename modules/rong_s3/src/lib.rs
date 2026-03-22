//! S3-compatible object storage module for RongJS.
//!
//! S3 client API inspired by Bun's `S3Client`, adapted for RongJS.

mod client;
mod config;
mod file;

pub use client::S3Client;
pub use config::S3Config;
pub use file::S3File;

use rong::*;

/// Register S3Client and S3File, expose `Rong.S3Client` and global `S3Client` constructor.
pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<S3File>()?;
    ctx.register_class::<S3Client>()?;

    let constructor = Class::get::<S3Client>(ctx)?;

    let rong = ctx.rong();
    rong.set("S3Client", constructor.clone())?;
    ctx.global().set("S3Client", constructor.clone())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    /// Spawn a local S3-compatible server backed by s3s-fs.
    /// Returns the `http://127.0.0.1:{port}` endpoint string.
    async fn spawn_s3_server() -> String {
        let tmp = tempfile::tempdir().expect("tempdir");
        let fs = s3s_fs::FileSystem::new(tmp.path()).expect("s3s-fs");

        // Pre-create the test bucket directory so S3 operations work immediately.
        std::fs::create_dir_all(tmp.path().join("test-bucket")).expect("create bucket dir");

        let mut auth = s3s::auth::SimpleAuth::new();
        auth.register("minioadmin".to_string(), "minioadmin".into());

        let mut builder = s3s::service::S3ServiceBuilder::new(fs);
        builder.set_auth(auth);
        let service = builder.build();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local_addr");

        // Leak tempdir so it lives for the duration of the process.
        let _tmp = Box::leak(Box::new(tmp));

        tokio::spawn(async move {
            let http_server =
                hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue;
                };
                let service = service.clone();
                let builder = http_server.clone();
                tokio::spawn(async move {
                    let _ = builder
                        .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                        .await;
                });
            }
        });

        format!("http://127.0.0.1:{}", addr.port())
    }

    #[test]
    fn test_s3() {
        let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root");
        std::env::set_current_dir(&workspace_root).expect("set cwd");

        async_run!(|ctx: JSContext| async move {
            // Bypass any system proxy for local connections
            unsafe {
                std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
            }

            // Start local mock S3 server
            let endpoint = spawn_s3_server().await;

            // Pass test config as JS globals (like redis does with TEST_REDIS_URL)
            ctx.global().set("TEST_S3_ENDPOINT", endpoint.as_str())?;
            ctx.global().set("TEST_S3_ACCESS_KEY", "minioadmin")?;
            ctx.global().set("TEST_S3_SECRET_KEY", "minioadmin")?;
            ctx.global().set("TEST_S3_BUCKET", "test-bucket")?;

            rong_console::init(&ctx)?;
            rong_assert::init(&ctx)?;
            init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "s3.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
