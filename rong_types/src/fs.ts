/**
 * File System module type definitions
 * Corresponds to: modules/rong_fs
 *
 * Provides APIs for file system operations including reading, writing, and directory management.
 * All file system APIs are mounted under the global `Rong` namespace.
 */

// ==================== Options Types ====================

/**
 * Options for reading files.
 */
export interface ReadFileOptions {
  /**
   * AbortSignal to cancel the read operation.
   * When aborted, the operation will throw an AbortError.
   *
   * @see {@link AbortSignal}
   * @example
   * ```typescript
   * const controller = new AbortController();
   * const promise = Rong.readTextFile('/file.txt', {
   *   signal: controller.signal
   * });
   * // Cancel after 1 second
   * setTimeout(() => controller.abort(), 1000);
   * ```
   */
  signal?: AbortSignal;
}

/**
 * Options for writing files.
 */
export interface WriteFileOptions {
  /**
   * Append to file instead of overwriting.
   * Cannot be used together with `createNew`.
   *
   * @default false
   */
  append?: boolean;

  /**
   * Create file only if it doesn't exist.
   * Throws AlreadyExistsError if the file already exists.
   * Cannot be used together with `append`.
   *
   * @default false
   * @throws {AlreadyExistsError} If file already exists
   */
  createNew?: boolean;

  /**
   * File permissions mode (Unix-like systems only).
   * Octal number (e.g., 0o644 for rw-r--r--).
   * Ignored on Windows.
   *
   * @default 0o666 (modified by process umask)
   * @platform unix
   * @example
   * ```typescript
   * await Rong.writeTextFile('/script.sh', '#!/bin/bash\necho hello', {
   *   mode: 0o755  // rwxr-xr-x
   * });
   * ```
   */
  mode?: number;

  /**
   * AbortSignal to cancel the write operation.
   *
   * @see {@link AbortSignal}
   */
  signal?: AbortSignal;
}

/**
 * Options for opening files.
 */
export interface FileOpenOptions {
  /**
   * Open file for reading.
   * @default false
   */
  read?: boolean;

  /**
   * Open file for writing.
   * @default false
   */
  write?: boolean;

  /**
   * Open file for appending (writes go to end of file).
   * @default false
   */
  append?: boolean;

  /**
   * Truncate file to 0 bytes when opening (if it exists).
   * Only valid when `write` is true.
   * @default false
   */
  truncate?: boolean;

  /**
   * Create file if it doesn't exist.
   * Only valid when `write` is true.
   * @default false
   */
  create?: boolean;

  /**
   * Create file only if it doesn't exist (exclusive creation).
   * Throws AlreadyExistsError if file exists.
   * Only valid when `write` is true.
   *
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
   * If false and parent doesn't exist, throws NotFoundError.
   *
   * @default false
   * @throws {NotFoundError} If parent doesn't exist and recursive is false
   * @example
   * ```typescript
   * // Create nested directories
   * await Rong.mkdir('/path/to/nested/dir', { recursive: true });
   * ```
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
   * If false and path is a non-empty directory, throws an error.
   *
   * @default false
   * @example
   * ```typescript
   * // Remove directory and all its contents
   * await Rong.remove('/path/to/dir', { recursive: true });
   * ```
   */
  recursive?: boolean;
}

/**
 * Options for setting file timestamps via {@link FsModule.utime}.
 */
export interface UTimeOptions {
  /**
   * Access time in milliseconds since Unix epoch.
   * If omitted, defaults to "now" (runtime-dependent).
   */
  accessed?: number;

  /**
   * Modified time in milliseconds since Unix epoch.
   * If omitted, defaults to "now" (runtime-dependent).
   */
  modified?: number;
}

// ==================== Information Types ====================

/**
 * File or directory metadata information.
 * Returned by `stat()` and `lstat()` functions.
 */
export interface FileInfo {
  /**
   * Whether this is a regular file.
   * Mutually exclusive with `isDirectory` and `isSymlink`.
   */
  readonly isFile: boolean;

  /**
   * Whether this is a directory.
   * Mutually exclusive with `isFile` and `isSymlink`.
   */
  readonly isDirectory: boolean;

  /**
   * Whether this is a symbolic link.
   * For symbolic links, use `lstat()` instead of `stat()`
   * to get link information rather than target information.
   */
  readonly isSymlink: boolean;

  /**
   * File size in bytes.
   * For directories, this is the size of the directory entry, not its contents.
   */
  readonly size: number;

  /**
   * Last modified time in milliseconds since Unix epoch.
   * May be undefined if not supported by the file system.
   */
  readonly modified?: number;

  /**
   * Last accessed time in milliseconds since Unix epoch.
   * May be undefined if not supported by the file system.
   */
  readonly accessed?: number;

  /**
   * Creation time in milliseconds since Unix epoch.
   * May be undefined if not supported by the file system.
   */
  readonly created?: number;

  /**
   * File permissions mode (Unix-like systems only).
   * Octal representation of file permissions.
   * Undefined on Windows.
   *
   * @platform unix
   */
  readonly mode?: number;
}

/**
 * Directory entry information.
 * Returned by `readDir()` function.
 */
export interface DirEntry {
  /**
   * Entry name (without directory path).
   * @example For `/path/to/file.txt`, name is `file.txt`
   */
  readonly name: string;

  /** Whether this entry is a regular file */
  readonly isFile: boolean;

  /** Whether this entry is a directory */
  readonly isDirectory: boolean;

  /** Whether this entry is a symbolic link */
  readonly isSymlink: boolean;
}

/**
 * Seek modes for file positioning.
 * Used with `FsFile.seek()` method.
 */
export enum SeekMode {
  /** Seek from start of file (absolute position) */
  Start = 0,
  /** Seek from current position (relative) */
  Current = 1,
  /** Seek from end of file (usually negative offset) */
  End = 2
}

// ==================== File Handle Interface ====================

/**
 * File handle for advanced file operations.
 * Obtained from `Rong.open()`.
 *
 * @example
 * ```typescript
 * const file = await Rong.open('/path/to/file.txt', {
 *   read: true,
 *   write: true
 * });
 *
 * try {
 *   const buffer = new ArrayBuffer(1024);
 *   const bytesRead = await file.read(buffer);
 *   console.log(`Read ${bytesRead} bytes`);
 * } finally {
 *   await file.close();
 * }
 * ```
 */
export interface FsFile {
  /**
   * Get file metadata.
   *
   * @returns Promise with file information
   * @throws {IOError} If stat operation fails
   * @example
   * ```typescript
   * const info = await file.stat();
   * console.log(`File size: ${info.size} bytes`);
   * ```
   */
  stat(): Promise<FileInfo>;

  /**
   * Read from file into buffer.
   * Reads from current file position and advances the position.
   *
   * @param buffer - ArrayBuffer to read into
   * @returns Promise with bytes read, or null on EOF
   * @throws {IOError} If read operation fails
   * @example
   * ```typescript
   * const buffer = new ArrayBuffer(4096);
   * const bytesRead = await file.read(buffer);
   * if (bytesRead === null) {
   *   console.log('Reached end of file');
   * } else {
   *   console.log(`Read ${bytesRead} bytes`);
   * }
   * ```
   */
  read(buffer: ArrayBuffer): Promise<number | null>;

  /**
   * Write buffer to file.
   * Writes at current file position and advances the position.
   *
   * @param buffer - ArrayBuffer to write
   * @returns Promise with number of bytes written
   * @throws {IOError} If write operation fails
   * @example
   * ```typescript
   * const data = new TextEncoder().encode('Hello World');
   * const bytesWritten = await file.write(data.buffer);
   * console.log(`Wrote ${bytesWritten} bytes`);
   * ```
   */
  write(buffer: ArrayBuffer): Promise<number>;

  /**
   * Sync file contents to disk.
   * Ensures all buffered writes are flushed to storage.
   *
   * @returns Promise that resolves when sync is complete
   * @throws {IOError} If sync operation fails
   */
  sync(): Promise<void>;

  /**
   * Truncate or extend file to specified length.
   *
   * @param len - Target length in bytes (default: 0)
   * @returns Promise that resolves when truncate is complete
   * @throws {IOError} If truncate operation fails
   * @example
   * ```typescript
   * // Truncate to 100 bytes
   * await file.truncate(100);
   *
   * // Truncate to 0 bytes (clear file)
   * await file.truncate();
   * ```
   */
  truncate(len?: number): Promise<void>;

  /**
   * Seek to position in file.
   *
   * @param offset - Byte offset
   * @param whence - Seek mode (Start, Current, or End)
   * @returns Promise with new absolute position
   * @throws {IOError} If seek operation fails
   * @example
   * ```typescript
   * // Seek to byte 100 from start
   * await file.seek(100, Rong.SeekMode.Start);
   *
   * // Seek forward 50 bytes from current position
   * await file.seek(50, Rong.SeekMode.Current);
   *
   * // Seek to 10 bytes before end
   * await file.seek(-10, Rong.SeekMode.End);
   * ```
   */
  seek(offset: number, whence?: SeekMode): Promise<number>;

  /**
   * Close file handle.
   * After closing, no further operations can be performed.
   * It's recommended to use try-finally to ensure files are closed.
   *
   * @returns Promise that resolves when file is closed
   * @example
   * ```typescript
   * const file = await Rong.open('/file.txt', { read: true });
   * try {
   *   // Use file...
   * } finally {
   *   await file.close();
   * }
   * ```
   */
  close(): Promise<void>;

  /**
   * Get ReadableStream for reading file contents.
   * The stream reads from the current file position.
   * Cannot be used simultaneously with direct read operations.
   *
   * @example
   * ```typescript
   * const file = await Rong.open('/file.txt', { read: true });
   * const readable = file.readable;
   *
   * for await (const chunk of readable) {
   *   console.log('Received chunk:', chunk.length, 'bytes');
   * }
   * ```
   */
  readonly readable: ReadableStream<Uint8Array>;

  /**
   * Get WritableStream for writing file contents.
   * The stream writes to the current file position.
   * Cannot be used simultaneously with direct write operations.
   *
   * @example
   * ```typescript
   * const file = await Rong.open('/file.txt', { write: true, create: true });
   * const writable = file.writable;
   * const writer = writable.getWriter();
   *
   * await writer.write(new TextEncoder().encode('Hello'));
   * await writer.close();
   * ```
   */
  readonly writable: WritableStream<Uint8Array>;
}

// ==================== Module Interface ====================

/**
 * File System module interface.
 * All operations are available under the global `Rong` namespace.
 */
export interface FsModule {
  // Read operations
  readTextFile(path: string, options?: ReadFileOptions): Promise<string>;
  readFile(path: string, options?: ReadFileOptions): Promise<ArrayBuffer>;

  // Write operations
  writeTextFile(path: string, text: string, options?: WriteFileOptions): Promise<void>;
  writeFile(path: string, data: ArrayBufferView, options?: WriteFileOptions): Promise<void>;
  copyFile(from: string, to: string): Promise<void>;
  truncate(path: string, len?: number): Promise<void>;

  // File operations
  open(path: string, options?: FileOpenOptions): Promise<FsFile>;

  // Directory operations
  mkdir(path: string, options?: MkdirOptions): Promise<void>;
  readDir(path: string): Promise<AsyncIterableIterator<DirEntry>>;
  stat(path: string): Promise<FileInfo>;
  lstat(path: string): Promise<FileInfo>;
  remove(path: string, options?: RemoveOptions): Promise<void>;
  chdir(path: string): Promise<void>;

  // Symlink operations
  symlink(target: string, path: string): Promise<void>;
  readlink(path: string): Promise<string>;

  // Permission operations
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

  // Path operations
  rename(oldPath: string, newPath: string): Promise<void>;
  realPath(path: string): Promise<string>;

  // Constants
  readonly SeekMode: typeof SeekMode;
}

// Note: File system APIs are declared under Rong namespace in global.d.ts
export {};
