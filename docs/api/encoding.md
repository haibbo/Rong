# Encoding

Text encoding/decoding and Base64 conversion.

## TextEncoder

```javascript
const encoder = new TextEncoder();
const bytes = encoder.encode("Hello World"); // Uint8Array
```

## TextDecoder

```javascript
const decoder = new TextDecoder();
const text = decoder.decode(new Uint8Array([72, 101, 108, 108, 111]));
// "Hello"
```

## Base64

```javascript
const encoded = btoa("Hello");   // "SGVsbG8="
const decoded = atob("SGVsbG8="); // "Hello"
```
