# DOMException

Web-standard exception type.

```javascript
const err = new DOMException("operation cancelled", "AbortError");
err.name;    // "AbortError"
err.message; // "operation cancelled"
```

## Error Names

| Name | Description |
|------|-------------|
| `AbortError` | Operation aborted |
| `NetworkError` | Network failure |
| `TimeoutError` | Operation timed out |
| `NotSupportedError` | Unsupported operation |
| `InvalidStateError` | Invalid state |
| `SyntaxError` | Syntax error |
| `SecurityError` | Security restriction |
| `QuotaExceededError` | Quota exceeded |
| `NotFoundError` | Not found |
| `DataCloneError` | Data clone failed |
| `InvalidAccessError` | Invalid access |
| `TypeMismatchError` | Type mismatch |
| `URLMismatchError` | URL mismatch |
