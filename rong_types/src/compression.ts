/**
 * Compression APIs mounted on the Rong namespace.
 * Corresponds to: modules/rong_compression
 */

export type RongCompressionInput = ArrayBuffer | ArrayBufferView;

export interface RongZstdCompressOptions {
  level?: number;
}

export interface RongGzipCompressOptions {
  level?: number;
}
