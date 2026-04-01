/**
 * Storage module type definitions
 * Corresponds to: modules/rong_storage
 *
 * IMPORTANT: The standard Rong runtime exposes a global `Storage` constructor.
 * It does not provide `Rong.Storage` or `Rong.storage.open(...)` by default.
 * This package does not redeclare the global `Storage` name because it would
 * conflict with the DOM `Storage` type from `lib.dom.d.ts`.
 */

export interface StorageInfo {
  /** Current storage size in bytes */
  currentSize: number;
  /** Size limit in bytes */
  limitSize: number;
  /** Number of keys */
  keyCount: number;
}

export interface Storage {
  /** Set a key-value pair */
  set(key: string, value: any): Promise<void>;

  /** Get value by key */
  get(key: string): Promise<any>;

  /** Delete a key */
  delete(key: string): Promise<void>;

  /** Clear all items */
  clear(): Promise<void>;

  /** Get all keys (returns a synchronous iterator wrapped in a Promise) */
  list(prefix?: string): Promise<IterableIterator<string>>;

  /** Get storage info */
  info(): Promise<StorageInfo>;
}

export interface StorageConstructor {
  /** Create a new Storage instance for the given database path */
  new(path: string, options?: StorageOptionsInput): Storage;
}

export interface StorageOptionsInput {
  maxKeySize?: number;
  maxValueSize?: number;
  maxDataSize?: number;
}

export {};
