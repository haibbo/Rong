/**
 * Buffer module type definitions (Blob and File)
 * Corresponds to: modules/rong_buffer
 */

export type BlobPart = Blob | ArrayBuffer | ArrayBufferView | string;

export interface BlobOptions {
  /** MIME type of the blob */
  type?: string;
  /** Line ending normalization: "transparent" (default) or "native" */
  endings?: 'transparent' | 'native';
}

export interface Blob {
  /** Blob size in bytes */
  readonly size: number;

  /** MIME type of the blob */
  readonly type: string;

  /** Create a slice of the blob */
  slice(start?: number, end?: number, contentType?: string): Blob;

  /** Get blob contents as ArrayBuffer */
  arrayBuffer(): Promise<ArrayBuffer>;

  /** Get blob contents as text */
  text(): Promise<string>;

  /** Get blob contents as Uint8Array */
  bytes(): Promise<Uint8Array>;
}

export interface BlobConstructor {
  new(blobParts?: BlobPart[], options?: BlobOptions): Blob;
  prototype: Blob;
}

export interface FileOptions extends BlobOptions {
  /** Last modified timestamp (milliseconds since epoch) */
  lastModified?: number;
}

export interface File extends Blob {
  /** File name */
  readonly name: string;

  /** Last modified timestamp (milliseconds since epoch) */
  readonly lastModified: number;
}

export interface FileConstructor {
  new(fileBits: BlobPart[], fileName: string, options?: FileOptions): File;
  prototype: File;
}

// Note: Blob and File are provided by the global environment
// These type definitions are for reference and extend the standard Web API
export {};
