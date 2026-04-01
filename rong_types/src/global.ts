/**
 * Global API declarations for Rong JavaScript Runtime
 *
 * This file declares all globally available APIs injected by the Rong runtime.
 * These declarations enable IDE autocomplete and TypeScript type checking.
 */

import type { AssertFunction } from './assert';
import type {
  RongGzipCompressOptions,
  RongCompressionInput,
  RongZstdCompressOptions,
} from './compression';
import type {
  RongEnvMap,
  RongOutputHandle,
  RongShellError,
  RongShellTag,
  RongReadableProcessStream,
  RongSpawnOptions,
  RongSpawnOptionsWithCmd,
  RongSubprocess,
  RongSyncSubprocess,
} from './command';
import type { RongSleepValue } from './timer';
import type {
  DirEntry,
  MkdirOptions,
  RemoveOptions,
  RongFile,
  SeekMode,
  UTimeOptions,
} from './fs';
import type { RedisClientConstructor } from './redis';
import type { SSEConstructor } from './sse';
import type { S3Client } from './s3';
import type { SQLite } from './sqlite';

declare global {
  /**
   * Rong runtime namespace - host APIs exposed by the Rong runtime
   */
  const Rong: {
    // Core file API
    file(path: string): RongFile;
    write(
      dest: string | RongFile,
      data: string | ArrayBufferView | ArrayBuffer | RongFile
    ): Promise<number>;

    // Directory operations
    mkdir(path: string, options?: MkdirOptions): Promise<void>;
    readDir(path: string): Promise<AsyncIterableIterator<DirEntry>>;
    remove(path: string, options?: RemoveOptions): Promise<void>;
    chdir(path: string): Promise<void>;

    // Symlink operations
    symlink(target: string, path: string): Promise<void>;
    readlink(path: string): Promise<string>;

    /** Change file permissions (Unix only) @platform unix */
    chmod(path: string, mode: number): Promise<void>;
    /** Change file ownership (Unix only) @platform unix */
    chown(path: string, uid: number, gid: number): Promise<void>;
    utime(path: string, options: UTimeOptions): Promise<void>;

    // Path operations
    rename(oldPath: string, newPath: string): Promise<void>;
    realPath(path: string): Promise<string>;
    readonly SeekMode: typeof SeekMode;

    // Runtime APIs
    readonly version: string;
    readonly revision: string;
    readonly argv: string[];
    readonly args: string[];
    readonly env: RongEnvMap;
    readonly stdin: RongReadableProcessStream;
    readonly stdout: RongOutputHandle;
    readonly stderr: RongOutputHandle;
    spawn(cmd: string[], options?: RongSpawnOptions): RongSubprocess;
    spawn(options: RongSpawnOptionsWithCmd): RongSubprocess;
    spawnSync(cmd: string[], options?: RongSpawnOptions): RongSyncSubprocess;
    spawnSync(options: RongSpawnOptionsWithCmd): RongSyncSubprocess;
    sleep(delay?: RongSleepValue): Promise<void>;
    sleepSync(delay?: number): void;
    zstdCompress(
      data: RongCompressionInput,
      options?: RongZstdCompressOptions
    ): Promise<Uint8Array>;
    zstdCompressSync(
      data: RongCompressionInput,
      options?: RongZstdCompressOptions
    ): Uint8Array;
    zstdDecompress(data: RongCompressionInput): Promise<Uint8Array>;
    zstdDecompressSync(data: RongCompressionInput): Uint8Array;
    gzip(
      data: RongCompressionInput,
      options?: RongGzipCompressOptions
    ): Promise<Uint8Array>;
    gzipSync(
      data: RongCompressionInput,
      options?: RongGzipCompressOptions
    ): Uint8Array;
    gunzip(data: RongCompressionInput): Promise<Uint8Array>;
    gunzipSync(data: RongCompressionInput): Uint8Array;
    readonly $: RongShellTag;
    readonly ShellError: {
      new (message: string): RongShellError;
      prototype: RongShellError;
    };
    readonly RedisClient: RedisClientConstructor;
    readonly S3Client: typeof S3Client;
    readonly SQLite: typeof SQLite;
    readonly SSE: SSEConstructor;
  };

  /**
   * Base64 decode - Decode base64 string to binary string
   */
  function atob(data: string): string;

  /**
   * Base64 encode - Encode binary string to base64
   */
  function btoa(data: string): string;

  /**
   * Assert function - Test assertions (Node.js compatible)
   */
  const assert: AssertFunction;

}

export {};
