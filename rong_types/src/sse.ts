export interface SSEOptions {
  headers?: Record<string, string>;
  requestTimeoutMs?: number;
  signal?: AbortSignal;
  reconnect?: {
    enabled?: boolean;
    maxRetries?: number;
    baseDelayMs?: number;
    maxDelayMs?: number;
  };
}

export interface SSEEvent {
  readonly type: string;
  readonly data: string;
  readonly id: string;
  readonly origin: string;
}

export interface SSE extends AsyncIterable<SSEEvent> {
  readonly url: string;
  close(): void;
  [Symbol.asyncIterator](): AsyncIterator<SSEEvent>;
}

export interface SSEConstructor {
  new (url: string, options?: SSEOptions): SSE;
}
