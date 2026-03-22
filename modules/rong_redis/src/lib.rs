//! # Rong Redis Module
//!
//! Redis client API inspired by Bun's RedisClient, adapted for RongJS.
//!
//! Provides `RedisClient` as a global class with promise-based methods
//! for strings, hashes, sets, lists, pub/sub, and raw commands.

use rong::*;

mod redis;
pub use redis::*;

/// Initialize the Redis module — exposes `RedisClient` globally.
pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<RedisClient>()?;
    ctx.register_class::<RedisSubscription>()?;

    ctx.eval::<()>(Source::from_bytes(
        r#"(function() {
            const proto = RedisClient.prototype;
            const _subscribe = proto.subscribe;
            proto.subscribe = function subscribe(channel, options) {
                return options === undefined
                    ? _subscribe.call(this, channel)
                    : _subscribe.call(this, channel, options);
            };
        })();"#,
    ))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;
    use std::time::Duration;

    /// Start a redis-server on a random port. Returns (url, child).
    /// The child is killed when dropped via `kill_on_drop(true)`.
    async fn start_test_redis() -> Result<(String, tokio::process::Child), String> {
        // Find a free port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("cannot bind: {}", e))?;
        let port = listener
            .local_addr()
            .map_err(|e| format!("no local addr: {}", e))?
            .port();
        drop(listener);

        // Try common redis-server locations
        let candidates = [
            "redis-server",
            "/usr/local/opt/redis/bin/redis-server",
            "/opt/homebrew/opt/redis/bin/redis-server",
            "/usr/bin/redis-server",
            "/usr/local/bin/redis-server",
        ];

        let bin = candidates.iter().find(|c| {
            std::process::Command::new(c)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok()
        });

        let bin = match bin {
            Some(b) => *b,
            None => return Err("redis-server not found".to_string()),
        };

        let child = tokio::process::Command::new(bin)
            .args([
                "--port",
                &port.to_string(),
                "--save",
                "",
                "--appendonly",
                "no",
                "--loglevel",
                "warning",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("failed to spawn redis-server: {}", e))?;

        // Wait for server to accept connections
        for _ in 0..50 {
            if tokio::net::TcpStream::connect(format!("127.0.0.1:{}", port))
                .await
                .is_ok()
            {
                let url = format!("redis://127.0.0.1:{}", port);
                return Ok((url, child));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err("redis-server did not start in time".to_string())
    }

    async fn setup_redis_env(
        ctx: &JSContext,
    ) -> Result<(String, tokio::process::Child), RongJSError> {
        let (url, child) = start_test_redis().await.map_err(|msg| {
            HostError::new(
                "E_TEST_SETUP",
                format!("Failed to start redis-server for rong_redis tests: {}", msg),
            )
        })?;

        ctx.global().set("TEST_REDIS_URL", url.as_str())?;
        rong_assert::init(ctx)?;
        rong_console::init(ctx)?;
        rong_abort::init(ctx)?;
        rong_timer::init(ctx)?;
        init(ctx)?;

        Ok((url, child))
    }

    #[test]
    fn test_redis() {
        async_run!(|ctx: JSContext| async move {
            let (_url, _child) = setup_redis_env(&ctx).await?;

            let passed = UnitJSRunner::load_script(&ctx, "redis.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }

    #[test]
    fn test_redis_namespace() {
        async_run!(|ctx: JSContext| async move {
            let (url, _child) = setup_redis_env(&ctx).await?;

            // Create a pre-configured client with namespace prefix from Rust,
            // then inject it as a global `redis` — JS never calls `new RedisClient`.
            let client = RedisClient::new(url, Some("app1:".to_string()));
            let js_client = Class::get::<RedisClient>(&ctx)?.instance(client);
            ctx.global().set("redis", js_client)?;

            let passed = UnitJSRunner::load_script(&ctx, "redis_namespace.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
