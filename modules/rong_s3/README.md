# rong_s3

S3-compatible object storage client. Exposed as global `S3Client`.

## JS APIs

- `S3Client` — global S3 client class
  - `new S3Client(options?)` — create a client
    - Options: `accessKeyId`, `secretAccessKey`, `bucket`, `region`, `endpoint`, `sessionToken`, `acl`, `virtualHostedStyle`
  - `file(path, options?)` — lazy `S3File` reference (no network request)
  - `write(path, data, options?)` — upload data, returns bytes written
  - `delete(path)` / `unlink(path)` — delete an object
  - `exists(path)` — check if an object exists
  - `size(path)` — get object size in bytes
  - `stat(path)` — get object metadata (`etag`, `lastModified`, `size`, `type`)
  - `presign(path, options?)` — generate a presigned URL
  - `list(options?)` — list objects (`prefix`, `maxKeys`, `startAfter`)
- `S3File` — lazy reference to an S3 object (via `client.file()`)
  - `text()` / `json()` / `bytes()` / `arrayBuffer()` — read
  - `write(data, options?)` — write
  - `slice(start, end)` — partial read reference
  - `exists()` / `stat()` / `delete()` / `unlink()` — metadata & delete
  - `presign(options?)` — presigned URL
  - `name` / `size` — getters

## Namespaced Injected Clients

When an `S3Client` is created from Rust with a non-empty `namespace_prefix`, JS code **cannot override S3 config fields** (`accessKeyId`, `secretAccessKey`, `sessionToken`, `region`, `endpoint`, `bucket`, `acl`, `virtualHostedStyle`) in method options. Attempting to do so throws `TypeError`. This prevents JS from escaping the intended bucket/credentials scope.

Allowed option fields per method:

| Method | Allowed options |
|--------|----------------|
| `file(path, options?)` | *(none)* |
| `write(path, data, options?)` | `type` |
| `presign(path, options?)` | `expiresIn`, `method` |
| `list(options?)` | `prefix`, `maxKeys`, `startAfter` |

## Rust API

- `S3Client::new(config, namespace_prefix)` — create a pre-configured client from Rust. The optional `namespace_prefix` is transparently prepended to all object keys and stripped from list results.
- `S3Config` — configuration struct with public fields (`access_key_id`, `secret_access_key`, `bucket`, `region`, `endpoint`, etc.)

To hide the JS constructor after init:

```rust
rong_s3::init(&ctx)?;
ctx.global().delete("S3Client")?;
```
