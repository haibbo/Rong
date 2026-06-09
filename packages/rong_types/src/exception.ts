/**
 * Exception module type definitions
 * Corresponds to: modules/rong_exception
 */

export type DOMExceptionName =
  | 'IndexSizeError'
  | 'DOMStringSizeError'
  | 'HierarchyRequestError'
  | 'InvalidCharacterError'
  | 'NoDataAllowedError'
  | 'NoModificationAllowedError'
  | 'NotFoundError'
  | 'NotSupportedError'
  | 'InUseAttributeError'
  | 'InvalidStateError'
  | 'SyntaxError'
  | 'InvalidModificationError'
  | 'NamespaceError'
  | 'InvalidAccessError'
  | 'ValidationError'
  | 'TypeMismatchError'
  | 'SecurityError'
  | 'NetworkError'
  | 'AbortError'
  | 'URLMismatchError'
  | 'QuotaExceededError'
  | 'TimeoutError'
  | 'InvalidNodeTypeError'
  | 'DataCloneError'
  | 'Error'
  // Node.js-style constant names accepted by Rong's DOMException implementation
  | 'INDEX_SIZE_ERR'
  | 'DOMSTRING_SIZE_ERR'
  | 'HIERARCHY_REQUEST_ERR'
  | 'INVALID_CHARACTER_ERR'
  | 'NO_DATA_ALLOWED_ERR'
  | 'NO_MODIFICATION_ALLOWED_ERR'
  | 'NOT_FOUND_ERR'
  | 'NOT_SUPPORTED_ERR'
  | 'INUSE_ATTRIBUTE_ERR'
  | 'INVALID_STATE_ERR'
  | 'SYNTAX_ERR'
  | 'INVALID_MODIFICATION_ERR'
  | 'NAMESPACE_ERR'
  | 'INVALID_ACCESS_ERR'
  | 'VALIDATION_ERR'
  | 'TYPE_MISMATCH_ERR'
  | 'SECURITY_ERR'
  | 'NETWORK_ERR'
  | 'ABORT_ERR'
  | 'URL_MISMATCH_ERR'
  | 'QUOTA_EXCEEDED_ERR'
  | 'TIMEOUT_ERR'
  | 'INVALID_NODE_TYPE_ERR'
  | 'DATA_CLONE_ERR'
  | 'ERROR';

export interface DOMException extends Error {
  /** Error name */
  readonly name: string;

  /** Error message */
  readonly message: string;

  /** Stack trace (currently returns "NotImplemented") */
  readonly stack: string;
}

export interface DOMExceptionConstructor {
  new(message?: string, name?: DOMExceptionName): DOMException;
  prototype: DOMException;
}

// Note: DOMException is provided by the global environment (Web API)
// These type definitions are for reference and extend the standard Web API
export {};
