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
  FileInfo,
  FileOpenOptions,
  FsFile,
  MkdirOptions,
  ReadFileOptions,
  RemoveOptions,
  SeekMode,
  UTimeOptions,
  WriteFileOptions,
} from './fs';
import type { PathModule } from './path';
import type { Process } from './process';
import type { StorageConstructor, StorageModule } from './storage';

declare global {
  /**
   * Rong runtime namespace - Core APIs for file system and storage
   */
  const Rong: {
    // File System APIs
    readTextFile(path: string, options?: ReadFileOptions): Promise<string>;
    readFile(path: string, options?: ReadFileOptions): Promise<ArrayBuffer>;
    writeTextFile(path: string, text: string, options?: WriteFileOptions): Promise<void>;
    writeFile(path: string, data: ArrayBufferView, options?: WriteFileOptions): Promise<void>;
    copyFile(from: string, to: string): Promise<void>;
    truncate(path: string, len?: number): Promise<void>;
    open(path: string, options?: FileOpenOptions): Promise<FsFile>;
    mkdir(path: string, options?: MkdirOptions): Promise<void>;
    readDir(path: string): Promise<AsyncIterableIterator<DirEntry>>;
    stat(path: string): Promise<FileInfo>;
    lstat(path: string): Promise<FileInfo>;
    remove(path: string, options?: RemoveOptions): Promise<void>;
    chdir(path: string): Promise<void>;
    symlink(target: string, path: string): Promise<void>;
    readlink(path: string): Promise<string>;
    /**
     * Change file permissions (Unix only)
     * @platform unix
     */
    chmod(path: string, mode: number): Promise<void>;
    /**
     * Change file ownership (Unix only)
     * @platform unix
     */
    chown(path: string, uid: number, gid: number): Promise<void>;
    utime(path: string, options: UTimeOptions): Promise<void>;
    rename(oldPath: string, newPath: string): Promise<void>;
    realPath(path: string): Promise<string>;
    readonly SeekMode: typeof SeekMode;

    // Storage
    readonly Storage: StorageConstructor;
    readonly storage: StorageModule;
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
   * Path module - Path manipulation utilities (Node.js compatible)
   */
  const path: PathModule;

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
