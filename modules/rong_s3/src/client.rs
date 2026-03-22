use crate::config::S3Config;
use crate::file::{S3File, resolve_body};
use rong::function::*;
use rong::*;
use std::rc::Rc;

fn s3_error(msg: impl Into<String>) -> RongJSError {
    HostError::new("ERR_S3", msg).with_name("S3Error").into()
}

/// S3-compatible object storage client.
#[js_export]
pub struct S3Client {
    pub(crate) config: Rc<S3Config>,
}

#[js_class]
impl S3Client {
    #[js_method(constructor)]
    fn new(options: Optional<JSObject>) -> JSResult<Self> {
        let config = match options.0 {
            Some(ref obj) => S3Config::from_js_options(obj)?,
            None => S3Config::default(),
        };
        Ok(Self {
            config: config.into_rc(),
        })
    }

    /// Lazy file reference — no network request.
    #[js_method]
    fn file(
        &self,
        ctx: JSContext,
        path: String,
        options: Optional<JSObject>,
    ) -> JSResult<JSObject> {
        let config = if let Some(ref obj) = options.0 {
            self.config.merge_js_options(obj)?.into_rc()
        } else {
            self.config.clone()
        };
        let file = S3File::create(config, path);
        Ok(Class::get::<S3File>(&ctx)?.instance(file))
    }

    #[js_method]
    async fn write(
        &self,
        path: String,
        data: JSValue,
        options: Optional<JSObject>,
    ) -> JSResult<f64> {
        let config = if let Some(ref obj) = options.0 {
            self.config.merge_js_options(obj)?
        } else {
            (*self.config).clone()
        };
        let bucket = config.create_bucket()?;
        let (content_bytes, content_type) = resolve_body(&data)?;
        let ct = if let Some(ref opts) = options.0 {
            opts.get::<_, String>("type").ok().or(content_type)
        } else {
            content_type
        };
        let ct_str = ct.as_deref().unwrap_or("application/octet-stream");

        bucket
            .put_object_with_content_type(&path, &content_bytes, ct_str)
            .await
            .map_err(|e| s3_error(format!("PUT {}: {}", path, e)))?;

        Ok(content_bytes.len() as f64)
    }

    #[js_method]
    async fn delete(&self, path: String) -> JSResult<()> {
        let bucket = self.config.create_bucket()?;
        bucket
            .delete_object(&path)
            .await
            .map_err(|e| s3_error(format!("DELETE {}: {}", path, e)))?;
        Ok(())
    }

    #[js_method]
    async fn unlink(&self, path: String) -> JSResult<()> {
        Self::delete(self, path).await
    }

    #[js_method]
    async fn exists(&self, path: String) -> JSResult<bool> {
        let bucket = self.config.create_bucket()?;
        match bucket.head_object(&path).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    #[js_method]
    async fn size(&self, path: String) -> JSResult<f64> {
        let bucket = self.config.create_bucket()?;
        let (head, _status) = bucket
            .head_object(&path)
            .await
            .map_err(|e| s3_error(format!("HEAD {}: {}", path, e)))?;
        Ok(head.content_length.unwrap_or(0) as f64)
    }

    #[js_method]
    async fn stat(&self, ctx: JSContext, path: String) -> JSResult<JSObject> {
        let bucket = self.config.create_bucket()?;
        let (head, _status) = bucket
            .head_object(&path)
            .await
            .map_err(|e| s3_error(format!("HEAD {}: {}", path, e)))?;

        let result = JSObject::new(&ctx);
        if let Some(etag) = head.e_tag {
            result.set("etag", etag)?;
        }
        if let Some(last_modified) = head.last_modified {
            result.set("lastModified", last_modified)?;
        }
        if let Some(ct) = head.content_type {
            result.set("type", ct)?;
        }
        result.set("size", head.content_length.unwrap_or(0) as f64)?;
        Ok(result)
    }

    #[js_method]
    async fn presign(&self, path: String, options: Optional<JSObject>) -> JSResult<String> {
        let config = if let Some(ref obj) = options.0 {
            self.config.merge_js_options(obj)?
        } else {
            (*self.config).clone()
        };
        let bucket = config.create_bucket()?;

        let expires_in = options
            .0
            .as_ref()
            .and_then(|o| o.get::<_, f64>("expiresIn").ok())
            .map(|v| v as u32)
            .unwrap_or(86400);

        let method = options
            .0
            .as_ref()
            .and_then(|o| o.get::<_, String>("method").ok())
            .unwrap_or_else(|| "GET".to_string());

        match method.to_uppercase().as_str() {
            "GET" => bucket
                .presign_get(&path, expires_in, None)
                .await
                .map_err(|e| s3_error(format!("presign GET: {}", e)).into()),
            "PUT" => bucket
                .presign_put(&path, expires_in, None, None)
                .await
                .map_err(|e| s3_error(format!("presign PUT: {}", e)).into()),
            "DELETE" => bucket
                .presign_delete(&path, expires_in)
                .await
                .map_err(|e| s3_error(format!("presign DELETE: {}", e)).into()),
            other => Err(HostError::new(
                "ERR_S3_INVALID_METHOD",
                format!("Unsupported presign method: {}", other),
            )
            .into()),
        }
    }

    #[js_method]
    async fn list(&self, ctx: JSContext, options: Optional<JSObject>) -> JSResult<JSObject> {
        let config = if let Some(ref obj) = options.0 {
            self.config.merge_js_options(obj)?
        } else {
            (*self.config).clone()
        };
        let bucket = config.create_bucket()?;

        let prefix = options
            .0
            .as_ref()
            .and_then(|o| o.get::<_, String>("prefix").ok())
            .unwrap_or_default();

        let max_keys = options
            .0
            .as_ref()
            .and_then(|o| o.get::<_, f64>("maxKeys").ok())
            .map(|v| v as usize);

        let start_after = options
            .0
            .as_ref()
            .and_then(|o| o.get::<_, String>("startAfter").ok());

        let results = bucket
            .list(prefix, None)
            .await
            .map_err(|e| s3_error(format!("LIST: {}", e)))?;

        let result_obj = JSObject::new(&ctx);
        let contents = JSArray::new(&ctx)?;
        let mut total_count = 0usize;
        let mut is_truncated = false;

        'outer: for page in &results {
            for obj in &page.contents {
                if let Some(ref after) = start_after {
                    if obj.key <= *after {
                        continue;
                    }
                }
                if let Some(max) = max_keys {
                    if total_count >= max {
                        is_truncated = true;
                        break 'outer;
                    }
                }

                let entry = JSObject::new(&ctx);
                entry.set("key", obj.key.as_str())?;
                entry.set("size", obj.size as f64)?;
                entry.set("lastModified", obj.last_modified.as_str())?;
                if let Some(ref etag) = obj.e_tag {
                    entry.set("etag", etag.as_str())?;
                }
                contents.push(JSValue::from(&ctx, entry))?;
                total_count += 1;
            }
        }

        result_obj.set("contents", contents)?;
        result_obj.set("isTruncated", is_truncated)?;
        Ok(result_obj)
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, _mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
    }
}
