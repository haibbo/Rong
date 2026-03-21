/**
 * File System module type definitions
 * Corresponds to: modules/rong_fs
 *
 * Core API:
 *   Rong.file(path) → RongFile (lazy file reference)
 *   Rong.write(dest, data) → Promise<number> (universal write)
 *
 * All directory/path/permission APIs remain under the global `Rong` namespace.
 */

// ==================== Options Types ====================

/**
 * Options for opening files via {@link RongFile.open}.
 */
export interface FileOpenOptions {
  /** Open file for reading. @default true */
  read?: boolean;
  /** Open file for writing. @default false */
  write?: boolean;
  /** Open file for appending (writes go to end of file). @default false */
  append?: boolean;
  /** Truncate file to 0 bytes when opening. Only valid when `write` is true. @default false */
  truncate?: boolean;
  /** Create file if it doesn't exist. Only valid when `write` is true. @default false */
  create?: boolean;
  /**
   * Create file only if it doesn't exist (exclusive creation).
   * @default false
   * @throws {AlreadyExistsError} If file already exists
   */
  createNew?: boolean;
  /**
   * File permissions mode (Unix-like systems only).
   * @default 0o666 (modified by process umask)
   * @platform unix
   */
  mode?: number;
}

/**
 * Options for creating directories.
 */
export interface MkdirOptions {
  /**
   * Create parent directories as needed.
   * @default false
   * @throws {NotFoundError} If parent doesn't exist and recursive is false
   */
  recursive?: boolean;
  /**
   * Directory permissions mode (Unix-like systems only).
   * @default 0o777 (modified by process umask)
   * @platform unix
   */
  mode?: number;
}

/**
 * Options for removing files and directories.
 */
export interface RemoveOptions {
  /**
   * Remove directories and their contents recursively.
   * @default false
   */
  recursive?: boolean;
}

/**
 * Options for setting file timestamps via {@link FsModule.utime}.
 */
export interface UTimeOptions {
  /** Access time in milliseconds since Unix epoch. */
  accessed?: number;
  /** Modified time in milliseconds since Unix epoch. */
  modified?: number;
}

/**
 * Options for creating a FileSink writer via {@link RongFile.writer}.
 */
export interface FileSinkOptions {
  /**
   * If true, open file in append mode. Default is truncate (overwrite).
   * @default false
   */
  append?: boolean;
  /**
   * File permissions mode (Unix-like systems only).
   * @platform unix
   */
  mode?: number;
}

// ==================== Information Types ====================

/**
 * File or directory metadata information.
 * Returned by `RongFile.stat()` and `RongFile.lstat()`.
 */
export interface FileInfo {
  readonly isFile: boolean;
  readonly isDirectory: boolean;
  readonly isSymlink: boolean;
  /** File size in bytes. */
  readonly size: number;
  /** Last modified time in milliseconds since Unix epoch. */
  readonly modified?: number;
  /** Last accessed time in milliseconds since Unix epoch. */
  readonly accessed?: number;
  /** Creation time in milliseconds since Unix epoch. */
  readonly created?: number;
  /** File permissions mode (Unix only). @platform unix */
  readonly mode?: number;
}

/**
 * Directory entry information.
 * Returned by `Rong.readDir()`.
 */
export interface DirEntry {
  /** Entry name (without directory path). */
  readonly name: string;
  readonly isFile: boolean;
  readonly isDirectory: boolean;
  readonly isSymlink: boolean;
}

/**
 * Seek modes for file positioning.
 * Used with `FileHandle.seek()` method.
 */
export enum SeekMode {
  /** Seek from start of file (absolute position) */
  Start = 0,
  /** Seek from current position (relative) */
  Current = 1,
  /** Seek from end of file (usually negative offset) */
  End = 2
}

// ==================== FileSink Interface ====================

/**
 * Incremental file writer with optional append support.
 * Obtained from `RongFile.writer()`.
 *
 * @example
 * ```typescript
 * const w = await Rong.file('/log.txt').writer({ append: true });
 * await w.write("line 1\n");
 * await w.write("line 2\n");
 * await w.flush();
 * await w.end();
 * ```
 */
export interface FileSink {
  /** Write data. Accepts string, TypedArray, or ArrayBuffer. Returns bytes written. */
  write(data: string | ArrayBufferView | ArrayBuffer): Promise<number>;
  /** Flush buffered data to disk. */
  flush(): Promise<void>;
  /** Flush and close the writer. */
  end(): Promise<void>;
}

// ==================== FileHandle Interface ====================

/**
 * File handle for low-level file operations.
 * Obtained from `RongFile.open()`.
 *
 * @example
 * ```typescript
 * const handle = await Rong.file('/path/to/file.txt').open({
 *   read: true,
 *   write: true
 * });
 *
 * try {
 *   const buffer = new ArrayBuffer(1024);
 *   const bytesRead = await handle.read(buffer);
 *   console.log(`Read ${bytesRead} bytes`);
 * } finally {
 *   await handle.close();
 * }
 * ```
 */
export interface FileHandle {
  stat(): Promise<FileInfo>;
  read(buffer: ArrayBuffer): Promise<number | null>;
  write(buffer: ArrayBuffer): Promise<number>;
  sync(): Promise<void>;
  truncate(len?: number): Promise<void>;
  seek(offset: number, whence?: SeekMode): Promise<number>;
  close(): Promise<void>;
  readonly readable: ReadableStream<Uint8Array>;
  readonly writable: WritableStream<Uint8Array>;
}

// ==================== RongFile Interface ====================

/**
 * Lazy file reference. Created by `Rong.file(path)`.
 * Does NOT touch the filesystem until a method is called.
 *
 * @example
 * ```typescript
 * const f = Rong.file('/data.json');
 *
 * // Convenient whole-file operations
 * const text = await f.text();
 * const data = await f.json();
 * const exists = await f.exists();
 *
 * // Low-level access when needed
 * const handle = await f.open({ read: true, write: true });
 * await handle.seek(100, Rong.SeekMode.Start);
 * await handle.close();
 * ```
 */
export interface RongFile {
  /** The original path passed to `Rong.file()`. */
  readonly name: string;

  /** Read file contents as a UTF-8 string. */
  text(): Promise<string>;
  /** Read file contents and parse as JSON. */
  json(): Promise<any>;
  /** Read file contents as Uint8Array. */
  bytes(): Promise<Uint8Array>;
  /** Read file contents as ArrayBuffer. */
  arrayBuffer(): Promise<ArrayBuffer>;
  /** Get a ReadableStream of file contents. */
  stream(): ReadableStream<Uint8Array>;

  /** Check if the file exists. */
  exists(): Promise<boolean>;
  /** Delete the file. */
  delete(): Promise<void>;

  /** Get file metadata. */
  stat(): Promise<FileInfo>;
  /** Get file metadata (does not follow symlinks). */
  lstat(): Promise<FileInfo>;

  /** Open a low-level file handle for seek/truncate/random-access. */
  open(options?: FileOpenOptions): Promise<FileHandle>;
  /** Create an incremental writer (default: truncate; use `{ append: true }` for append). */
  writer(options?: FileSinkOptions): Promise<FileSink>;
}

// ==================== Module Interface ====================

/**
 * File System module interface.
 * All operations are available under the global `Rong` namespace.
 */
export interface FsModule {
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

  // Permission operations
  /** Change file permissions (Unix only) @platform unix */
  chmod(path: string, mode: number): Promise<void>;
  /** Change file ownership (Unix only) @platform unix */
  chown(path: string, uid: number, gid: number): Promise<void>;
  utime(path: string, options: UTimeOptions): Promise<void>;

  // Path operations
  rename(oldPath: string, newPath: string): Promise<void>;
  realPath(path: string): Promise<string>;

  // Constants
  readonly SeekMode: typeof SeekMode;
}

export {};
