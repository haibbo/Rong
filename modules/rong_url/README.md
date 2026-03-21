# rong_url

URL parsing and manipulation following the Web API standard.

## JS APIs

- `URL` — URL class
  - `new URL(url, base?)` — parse a URL string
  - Properties: `href`, `origin`, `protocol`, `host`, `hostname`, `port`, `pathname`, `search`, `hash`, `username`, `password`
  - `searchParams` — associated `URLSearchParams` instance
  - `toString()`, `toJSON()` — serialize back to string
- `URLSearchParams` — query string class
  - `new URLSearchParams(init?)` — create from string, array of pairs, or object
  - `get(name)`, `getAll(name)`, `has(name)`, `set(name, value)`, `append(name, value)`, `delete(name)`
  - `sort()` — sort parameters by name
  - `entries()`, `keys()`, `values()`, `forEach()`
  - `size` — number of parameters
  - `toString()` — serialize to query string
