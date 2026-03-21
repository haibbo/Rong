# HTTP — fetch / Request / Response / SSE

Web-standard Fetch API and Server-Sent Events.

## fetch

```javascript
const response = await fetch("https://api.example.com/data");
const json = await response.json();

// POST
const response = await fetch("https://api.example.com/submit", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ name: "rong" }),
});
```

### Signature

```typescript
fetch(input: string | Request | URL, init?: RequestInit): Promise<Response>
```

### RequestInit

| Property | Type | Description |
|----------|------|-------------|
| `method` | `string` | HTTP method, default `"GET"` |
| `headers` | `Headers \| Record<string, string> \| [string, string][]` | Request headers |
| `body` | `string \| ArrayBuffer \| Uint8Array \| Blob \| FormData \| URLSearchParams \| ReadableStream` | Request body |
| `redirect` | `"follow" \| "error" \| "manual"` | Redirect policy |
| `signal` | `AbortSignal` | Cancellation signal |

---

## Request

```javascript
const req = new Request("https://api.example.com", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ key: "value" }),
});

const req2 = new Request(req); // clone from existing
```

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `url` | `string` | Request URL |
| `method` | `string` | HTTP method |
| `headers` | `Headers` | Request headers |
| `body` | `ReadableStream \| null` | Body stream |
| `redirect` | `string` | Redirect policy |
| `signal` | `AbortSignal` | Cancellation signal |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `clone()` | `Request` | Clone the request |
| `text()` | `Promise<string>` | Read body as text |
| `json()` | `Promise<any>` | Read body as JSON |
| `arrayBuffer()` | `Promise<ArrayBuffer>` | Read body as binary |
| `blob()` | `Promise<Blob>` | Read body as Blob |

---

## Response

```javascript
// Manual construction
const res = new Response("Hello", {
  status: 200,
  headers: { "Content-Type": "text/plain" },
});

// From fetch
const res = await fetch(url);
if (res.ok) {
  const data = await res.json();
}
```

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `url` | `string` | Response URL |
| `status` | `number` | HTTP status code |
| `statusText` | `string` | Status text |
| `ok` | `boolean` | Status 200-299 |
| `headers` | `Headers` | Response headers |
| `body` | `ReadableStream \| null` | Body stream |
| `bodyUsed` | `boolean` | Whether body is consumed |
| `redirected` | `boolean` | Whether redirected |
| `type` | `string` | Response type |

### Methods

| Method | Returns |
|--------|---------|
| `text()` | `Promise<string>` |
| `json()` | `Promise<any>` |
| `arrayBuffer()` | `Promise<ArrayBuffer>` |
| `blob()` | `Promise<Blob>` |
| `formData()` | `Promise<FormData>` |
| `clone()` | `Response` |

---

## Headers

```javascript
const h = new Headers({ "Content-Type": "application/json" });
const h2 = new Headers([["Accept", "text/html"], ["Accept", "application/json"]]);
const h3 = new Headers(existingHeaders);

h.set("Authorization", "Bearer token");
h.append("Accept", "text/html");
h.get("Content-Type"); // "application/json"
h.has("Authorization"); // true
h.delete("Authorization");

for (const [name, value] of h.entries()) {
  console.log(`${name}: ${value}`);
}
```

| Method | Description |
|--------|-------------|
| `get(name)` | Get value |
| `set(name, value)` | Set value |
| `append(name, value)` | Append value |
| `delete(name)` | Delete |
| `has(name)` | Check existence |
| `entries()` | Iterate `[name, value]` |
| `keys()` | Iterate names |
| `values()` | Iterate values |
| `forEach(fn)` | For each entry |

---

## SSE (Server-Sent Events)

```javascript
const sse = new SSE("https://api.example.com/events", {
  headers: { Authorization: "Bearer token" },
  signal: AbortSignal.timeout(60000),
  reconnect: {
    enabled: true,
    baseDelayMs: 1000,
    maxDelayMs: 30000,
    maxRetries: 10,
  },
  requestTimeoutMs: 60000,
});

for await (const event of sse) {
  console.log(event.type, event.data);
  if (done) break; // triggers close + cleanup
}

// Manual close
sse.close();
```

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `url` | `string` | Connection URL |

### Methods

| Method | Description |
|--------|-------------|
| `close()` | Close connection |

### Options

| Option | Type | Description |
|--------|------|-------------|
| `headers` | `Record<string, string>` | Custom request headers |
| `signal` | `AbortSignal` | Cancellation signal |
| `requestTimeoutMs` | `number` | Request timeout in milliseconds |
| `reconnect` | `object` | Reconnect policy (`enabled`, `maxRetries`, `baseDelayMs`, `maxDelayMs`) |

### Yielded event shape

| Property | Type |
|----------|------|
| `type` | `string` |
| `data` | `string` |
| `id` | `string` |
| `origin` | `string` |
