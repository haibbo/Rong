#ifndef QJS_H
#define QJS_H

#include "quickjs.h"

JSValue QJS_NewBool(JSContext *ctx, JS_BOOL val);
JSValue QJS_NewInt32(JSContext *ctx, int32_t val);
JSValue QJS_NewFloat64(JSContext *ctx, double val);

JSValue QJS_NewInt64(JSContext *ctx, int64_t val);
JSValue QJS_NewUint32(JSContext *ctx, uint32_t val);
JS_BOOL QJS_IsNumber(JSContext *ctx, JSValue v);
JS_BOOL QJS_IsBigInt(JSContext *ctx, JSValue v);
JS_BOOL QJS_IsBool(JSContext *ctx, JSValue v);
JS_BOOL QJS_IsUndefined(JSContext *ctx, JSValue v);
JS_BOOL QJS_IsException(JSContext *ctx, JSValue v);
JS_BOOL QJS_IsString(JSContext *ctx, JSValue v);
JS_BOOL QJS_IsSymbol(JSContext *ctx, JSValue v);
JS_BOOL QJS_IsObject(JSContext *ctx, JSValue v);

JSValue QJS_RunScript(JSContext *ctx, char *script, int len);
void QJS_RunJobs(JSRuntime *rt);

#endif
