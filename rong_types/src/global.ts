/**
 * Global API declarations for Rong JavaScript Runtime
 *
 * This file declares all globally available APIs injected by the Rong runtime.
 * These declarations enable IDE autocomplete and TypeScript type checking.
 */

import type { AssertFunction } from './assert';
import type { ChildProcessModule } from './child_process';
import type {
  DirEntry,
  FileOpenOptions,
  MkdirOptions,
  RemoveOptions,
  RongFile,
  SeekMode,
  UTimeOptions,
} from './fs';
import type { Process } from './process';
import type { RedisClientConstructor } from './redis';

import type { SSEConstructor } from './sse';
import type { StorageConstructor, StorageModule } from './storage';
import type { RongWorkerConstructor } from './worker';

declare global {
  /**
   * Rong runtime namespace - Core APIs for file system and storage
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

    // Storage
    readonly Storage: StorageConstructor;
    readonly storage: StorageModule;

    // Database / cache clients
    readonly RedisClient: RedisClientConstructor;
    readonly S3Client: typeof import('./s3').S3Client;

  };

  /**
   * Process object - Access to process information and environment
   */
  const process: Process;

  /**
   * Child Process module - Node.js compatible child process spawning (globalThis.child_process)
   */
  const child_process: ChildProcessModule;

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

  /**
   * RedisClient - Async Redis client
   */
  const RedisClient: RedisClientConstructor;

  /**
   * S3Client - S3-compatible object storage client
   */
  const S3Client: typeof import('./s3').S3Client;

  /**
   * SQLite - Embedded SQLite database (sync API)
   */
  const SQLite: typeof import('./sqlite').SQLite;

  /**
   * SSE - Server-Sent Events async iterator client
   */
  const SSE: SSEConstructor;

  /**
   * Worker - Web Workers for running JS in dedicated threads
   */
  const Worker: RongWorkerConstructor;
}

export {};
