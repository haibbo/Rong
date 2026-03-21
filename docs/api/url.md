# URL & URLSearchParams

Web-standard URL parsing and query parameter manipulation.

## URL

```javascript
const url = new URL("https://example.com:8080/path?q=1#hash");

url.protocol;    // "https:"
url.hostname;    // "example.com"
url.port;        // "8080"
url.host;        // "example.com:8080"
url.pathname;    // "/path"
url.search;      // "?q=1"
url.hash;        // "#hash"
url.origin;      // "https://example.com:8080"
url.username;    // ""
url.password;    // ""

url.toString();  // full URL string
url.toJSON();    // same as toString()
```

### Relative URL

```javascript
const url = new URL("/api/users", "https://example.com");
url.href; // "https://example.com/api/users"
```

### Modify Properties

```javascript
url.pathname = "/new-path";
url.search = "?key=value";
```

## URLSearchParams

```javascript
// From string
const params = new URLSearchParams("foo=1&bar=2");

// From object
const params = new URLSearchParams({ foo: "1", bar: "2" });

// From array
const params = new URLSearchParams([["foo", "1"], ["bar", "2"]]);
```

### Methods

```javascript
params.get("foo");       // "1"
params.getAll("foo");    // ["1"]
params.has("foo");       // true
params.set("foo", "3");
params.append("baz", "4");
params.delete("bar");
params.sort();

params.keys();           // ["baz", "foo"]
params.values();         // ["4", "3"]
params.entries();        // [["baz", "4"], ["foo", "3"]]
params.toString();       // "baz=4&foo=3"
params.size;             // 2

params.forEach((value, key) => {
  console.log(key, value);
});
```
