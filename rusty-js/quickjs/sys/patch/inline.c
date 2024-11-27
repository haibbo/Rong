//#include "quickjs.h"
#include "qjs.h"

// copy some inline functions to help generate binding
// and add JSContxt as input parameter if it does not have
// rename prefix JS to QJS to avoid conflicting type

JSValue QJS_NewBool(JSContext *ctx, JS_BOOL val)
{
    (void)&ctx;
    return JS_MKVAL(JS_TAG_BOOL, (val != 0));
}

JSValue QJS_NewInt32(JSContext *ctx, int32_t val)
{
    (void)&ctx;
    return JS_MKVAL(JS_TAG_INT, val);
}

JSValue QJS_NewFloat64(JSContext *ctx, double val)
{
    (void)&ctx;
    return __JS_NewFloat64(val);
}

JSValue QJS_NewInt64(JSContext *ctx, int64_t val)
{
    JSValue v;
    if (val >= INT32_MIN && val <= INT32_MAX) {
        v = JS_NewInt32(ctx, (int32_t)val);
    } else {
        v = JS_NewFloat64(ctx, (double)val);
    }
    return v;
}

JSValue QJS_NewUint32(JSContext *ctx, uint32_t val)
{
    JSValue v;
    if (val <= INT32_MAX) {
        v = JS_NewInt32(ctx, (int32_t)val);
    } else {
        v = JS_NewFloat64(ctx, (double)val);
    }
    return v;
}

int QJS_ToUint32(JSContext *ctx, uint32_t *pres, JSValue val)
{
    return JS_ToInt32(ctx, (int32_t*)pres, val);
}


JS_BOOL QJS_IsNumber(JSContext *ctx, JSValue v)
{
    (void)&ctx;
    int tag = JS_VALUE_GET_TAG(v);
    return tag == JS_TAG_INT || JS_TAG_IS_FLOAT64(tag);
}

JS_BOOL QJS_IsBigInt(JSContext *ctx, JSValue v)
{
    (void)&ctx;
    return JS_VALUE_GET_TAG(v) == JS_TAG_BIG_INT;
}

JS_BOOL QJS_IsBool(JSContext *ctx, JSValue v)
{
    (void)&ctx;
    return JS_VALUE_GET_TAG(v) == JS_TAG_BOOL;
}


JS_BOOL QJS_IsUndefined(JSContext *ctx, JSValue v)
{
    return JS_VALUE_GET_TAG(v) == JS_TAG_UNDEFINED;
}

JS_BOOL QJS_IsException(JSContext *ctx, JSValue v)
{
    return JS_VALUE_GET_TAG(v) == JS_TAG_EXCEPTION;
}

JS_BOOL QJS_IsNull(JSContext *ctx, JSValue v)
{
    return JS_VALUE_GET_TAG(v) == JS_TAG_NULL;
}


JS_BOOL QJS_IsString(JSContext *ctx, JSValue v)
{
    return JS_VALUE_GET_TAG(v) == JS_TAG_STRING;
}

JS_BOOL QJS_IsSymbol(JSContext *ctx, JSValue v)
{
    return JS_VALUE_GET_TAG(v) == JS_TAG_SYMBOL;
}

JS_BOOL QJS_IsObject(JSContext *ctx, JSValue v)
{
    return JS_VALUE_GET_TAG(v) == JS_TAG_OBJECT;
}
