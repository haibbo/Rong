/**
 * SQLite module type definitions.
 * Corresponds to: modules/rong_sqlite
 *
 * Sync API — all operations are synchronous (no Promises).
 */

// ==================== Result Types ====================

/**
 * Result of a write operation (INSERT, UPDATE, DELETE).
 */
export interface RunResult {
  /** Number of rows changed. */
  changes: number;
  /** Row ID of the last inserted row. Large values are returned as `bigint`. */
  lastInsertRowid: number | bigint;
}

// ==================== Statement Interface ====================

/**
 * Prepared SQL statement for repeated execution.
 * Created via `db.prepare(sql)`.
 *
 * @example
 * ```typescript
 * const stmt = db.prepare("SELECT * FROM users WHERE age > ?");
 * const rows = stmt.all([18]);
 * const first = stmt.get([18]);
 * stmt.finalize();
 * ```
 */
export interface Statement {
  /** The SQL text of this statement. */
  readonly sql: string;

  /** Execute and return `{ changes, lastInsertRowid }`. */
  run(params?: SQLiteParams): RunResult;

  /** Execute and return all matching rows as array of objects. */
  all(params?: SQLiteParams): Record<string, any>[];

  /** Execute and return the first matching row, or `null`. */
  get(params?: SQLiteParams): Record<string, any> | null;

  /** Execute and return all rows as arrays of column values. */
  values(params?: SQLiteParams): any[][];

  /** Mark the statement as finalized. Further calls will throw. */
  finalize(): void;
}

// ==================== Database Interface ====================

/** Supported parameter types for SQLite queries. */
export type SQLiteParam = null | boolean | number | bigint | string | ArrayBuffer | Uint8Array;
export type SQLiteParams = SQLiteParam[];

/**
 * SQLite database connection.
 *
 * @example
 * ```typescript
 * const db = new SQLite("mydb.sqlite");
 *
 * db.exec("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)");
 * db.run("INSERT INTO users (name, age) VALUES (?, ?)", ["Alice", 30]);
 *
 * const rows = db.query("SELECT * FROM users WHERE age > ?", [18]);
 * console.log(rows);
 *
 * db.close();
 * ```
 */
export declare class SQLite {
  /**
   * Open a SQLite database.
   * @param filename - Path to the database file, or `":memory:"` for in-memory. Defaults to `":memory:"`.
   */
  constructor(filename?: string);

  /** The filename used to open the database. */
  readonly filename: string;

  /** Whether the database is currently inside a transaction. */
  readonly inTransaction: boolean;

  /**
   * Execute one or more SQL statements (no parameters, no return value).
   * Use for DDL / schema setup.
   */
  exec(sql: string): void;

  /**
   * Execute a single statement with optional parameters.
   * Returns `{ changes, lastInsertRowid }`.
   */
  run(sql: string, params?: SQLiteParams): RunResult;

  /**
   * Execute a query and return all matching rows as an array of objects.
   */
  query(sql: string, params?: SQLiteParams): Record<string, any>[];

  /**
   * Create a prepared statement for repeated execution.
   */
  prepare(sql: string): Statement;

  /**
   * Run a function inside a transaction.
   * Commits on success, rolls back if the callback throws.
   */
  transaction(callback: () => void): void;

  /** Close the database connection. */
  close(): void;
}

export {};
