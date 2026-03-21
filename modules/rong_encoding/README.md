# rong_encoding

Implements the Web Encoding API for text encoding and decoding.

## JS APIs

- `TextEncoder` — encodes strings to UTF-8 `Uint8Array`
  - `encode(input)` — encode a string
  - `encodeInto(input, destination)` — encode into an existing buffer
- `TextDecoder` — decodes byte sequences to strings
  - `new TextDecoder(label?, options?)` — create a decoder for a given encoding
  - `decode(input?, options?)` — decode bytes to a string
