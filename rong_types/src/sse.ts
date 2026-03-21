export interface EventSourceEventMap {
  open: Event;
  message: MessageEvent;
  error: Event;
}

export interface EventSourceInit {
  headers?: Record<string, string>;
  requestTimeoutMs?: number;
  reconnect?: {
    enabled?: boolean;
    maxRetries?: number;
    baseDelayMs?: number;
    maxDelayMs?: number;
  };
}

export interface MessageEvent {
  readonly type: string;
  readonly data: string;
  readonly lastEventId: string;
  readonly origin: string;
}

export interface EventSource {
  readonly url: string;
  readonly readyState: number;
  readonly lastEventId: string;
  readonly CONNECTING: 0;
  readonly OPEN: 1;
  readonly CLOSED: 2;
  onopen: ((event: Event) => void) | null;
  onmessage: ((event: MessageEvent) => void) | null;
  onerror: ((event: Event) => void) | null;
  close(): void;
  addEventListener(type: string, listener: (event: MessageEvent) => void): void;
  removeEventListener(type: string, listener: (event: MessageEvent) => void): void;
}

export interface EventSourceConstructor {
  new (url: string, options?: EventSourceInit): EventSource;
  readonly CONNECTING: 0;
  readonly OPEN: 1;
  readonly CLOSED: 2;
}
