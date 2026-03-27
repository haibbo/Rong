#include "qjs.h"

// copy some inline functions to help generate binding
// and add JSContxt as input parameter if it does not have
// rename prefix JS to QJS to avoid conflicting type

JSValue QJS_NewBool(JSContext *ctx, bool val)
{
    return JS_NewBool(ctx, val);
}

JSValue QJS_NewInt32(JSContext *ctx, int32_t val)
{
    return JS_NewInt32(ctx, val);
}

JSValue QJS_NewFloat64(JSContext *ctx, double val)
{
    return JS_NewFloat64(ctx, val);
}

JSValue QJS_NewUint32(JSContext *ctx, uint32_t val)
{
    return JS_NewUint32(ctx, val);
}

int QJS_ToUint32(JSContext *ctx, uint32_t *pres, JSValue val)
{
    return JS_ToUint32(ctx, pres, val);
}


bool QJS_IsNumber(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsNumber(v);
}

bool QJS_IsBigInt(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsBigInt(v);
}

bool QJS_IsBool(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsBool(v);
}


bool QJS_IsUndefined(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsUndefined(v);
}

JSValue QJS_NewUndefined(JSContext *ctx)
{
    (void)ctx;
    return JS_UNDEFINED;
}

JSValue QJS_NewNull(JSContext *ctx)
{
    (void)ctx;
    return JS_NULL;
}


bool QJS_IsException(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsException(v);
}

bool QJS_IsNull(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsNull(v);
}


bool QJS_IsString(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsString(v);
}

bool QJS_IsSymbol(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsSymbol(v);
}

bool QJS_IsObject(JSContext *ctx, JSValue v)
{
    (void)ctx;
    return JS_IsObject(v);
}
