/**
 * Storage module type definitions
 * Corresponds to: modules/rong_storage
 *
 * IMPORTANT: Storage is accessed via `Rong.storage.open(path)` or `new Rong.Storage(path)`
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

export interface StorageModule {
  /**
   * Open a storage database at the given path
   * @param path - Path to the database file
   * @returns Storage instance
   *
   * @example
   * ```typescript
   * const storage = await Rong.storage.open('/path/to/db.sqlite');
   * await storage.set('key', 'value');
   * ```
   */
  open(path: string, options?: StorageOptionsInput): Promise<Storage>;
}

// Note: Storage is accessed via Rong.storage.open() or new Rong.Storage()
export {};
