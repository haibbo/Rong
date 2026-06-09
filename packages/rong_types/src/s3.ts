/**
 * S3-compatible object storage module type definitions.
 * Corresponds to: modules/rong_s3
 *
 * Core API:
 *   new Rong.S3Client(options?) → S3Client
 *   client.file(path) → S3File (lazy reference, no network)
 */

// ==================== Options Types ====================

/**
 * S3 client configuration options.
 * All fields are optional at construction time, but actual requests
 * require accessKeyId, secretAccessKey, and bucket.
 */
export interface S3ClientOptions {
  /** AWS access key ID. */
  accessKeyId?: string;
  /** AWS secret access key. */
  secretAccessKey?: string;
  /** AWS session token (STS). */
  sessionToken?: string;
  /** AWS region. @default "us-east-1" */
  region?: string;
  /** Custom endpoint URL (for S3-compatible services). */
  endpoint?: string;
  /** Bucket name. */
  bucket?: string;
  /** Default ACL for uploads (e.g. "public-read"). */
  acl?: string;
  /** Use virtual-hosted-style URLs instead of path-style. @default false */
  virtualHostedStyle?: boolean;
}

/**
 * Options for presigning URLs.
 */
export interface S3PresignOptions {
  /** Expiration in seconds. @default 86400 (24 hours) */
  expiresIn?: number;
  /** HTTP method. @default "GET" */
  method?: "GET" | "PUT" | "DELETE";
}

/**
 * Options for write operations.
 */
export interface S3WriteOptions {
  /** Content-Type header. @default "application/octet-stream" */
  type?: string;
}

/**
 * Options for list operations.
 */
export interface S3ListOptions {
  /** Filter objects by key prefix. */
  prefix?: string;
  /** Maximum number of keys to return. */
  maxKeys?: number;
  /** Start listing after this key (for pagination). */
  startAfter?: string;
}

// ==================== Result Types ====================

/**
 * Object metadata returned by `stat()`.
 */
export interface S3StatResult {
  /** ETag of the object. */
  etag?: string;
  /** Last modified timestamp (ISO 8601 string). */
  lastModified?: string;
  /** Object size in bytes. */
  size: number;
  /** Content-Type of the object. */
  type?: string;
}

/**
 * Single object entry in a list result.
 */
export interface S3ListEntry {
  /** Object key. */
  key: string;
  /** Object size in bytes. */
  size: number;
  /** Last modified timestamp (ISO 8601 string). */
  lastModified: string;
  /** ETag of the object. */
  etag?: string;
}

/**
 * Result of a list operation.
 */
export interface S3ListResult {
  /** List of matching objects. */
  contents: S3ListEntry[];
  /** Whether there are more results (use `startAfter` to paginate). */
  isTruncated: boolean;
}

// ==================== S3File Interface ====================

/**
 * Lazy reference to an S3 object. No network request on creation.
 * Obtained from `S3Client.file()`.
 *
 * @example
 * ```typescript
 * const file = client.file("data.json");
 * const text = await file.text();
 * const data = await file.json();
 * await file.write("new content");
 * await file.delete();
 * ```
 */
export interface S3File {
  /** The object key. */
  readonly name: string;
  /** Always NaN — use stat().size for object size. */
  readonly size: number;

  /** Read object contents as UTF-8 string. */
  text(): Promise<string>;
  /** Read object contents and parse as JSON. */
  json(): Promise<any>;
  /** Read object contents as ArrayBuffer. */
  bytes(): Promise<ArrayBuffer>;
  /** Read object contents as ArrayBuffer (alias for bytes). */
  arrayBuffer(): Promise<ArrayBuffer>;

  /** Write data to this S3 object. Returns bytes written. */
  write(data: string | ArrayBuffer | Uint8Array, options?: S3WriteOptions): Promise<number>;

  /** Delete this object. */
  delete(): Promise<void>;
  /** Delete this object (alias for delete). */
  unlink(): Promise<void>;

  /** Check if this object exists. */
  exists(): Promise<boolean>;
  /** Get object metadata (HEAD request). */
  stat(): Promise<S3StatResult>;

  /** Generate a presigned URL for this object. */
  presign(options?: S3PresignOptions): Promise<string>;

  /** Create a new S3File referencing a byte range of this object. */
  slice(start: number, end?: number): S3File;
}

// ==================== S3Client Interface ====================

/**
 * S3-compatible object storage client.
 *
 * @example
 * ```typescript
 * const client = new Rong.S3Client({
 *   accessKeyId: "...",
 *   secretAccessKey: "...",
 *   bucket: "my-bucket",
 *   endpoint: "https://s3.us-east-1.amazonaws.com",
 * });
 *
 * await client.write("hello.txt", "Hello World!");
 * const file = client.file("hello.txt");
 * const text = await file.text();
 * ```
 */
export declare class S3Client {
  constructor(options?: S3ClientOptions);

  /** Create a lazy S3File reference. No network request. */
  file(path: string, options?: S3ClientOptions): S3File;

  /** Write data to an S3 object. Returns bytes written. */
  write(path: string, data: string | ArrayBuffer | Uint8Array, options?: S3WriteOptions & S3ClientOptions): Promise<number>;

  /** Delete an object. */
  delete(path: string): Promise<void>;
  /** Delete an object (alias for delete). */
  unlink(path: string): Promise<void>;

  /** Check if an object exists. */
  exists(path: string): Promise<boolean>;
  /** Get object size in bytes. */
  size(path: string): Promise<number>;
  /** Get object metadata (HEAD request). */
  stat(path: string): Promise<S3StatResult>;

  /** Generate a presigned URL. */
  presign(path: string, options?: S3PresignOptions & S3ClientOptions): Promise<string>;

  /** List objects in the bucket. */
  list(options?: S3ListOptions & S3ClientOptions): Promise<S3ListResult>;
}

export {};
