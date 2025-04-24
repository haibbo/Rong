#ifndef QJS_H
#define QJS_H

#include "quickjs.h"
#include <stdbool.h>

JSValue QJS_NewBool(JSContext *ctx, bool val);

JSValue QJS_NewInt32(JSContext *ctx, int32_t val);
int QJS_ToUint32(JSContext *ctx, uint32_t *pres, JSValue val);

JSValue QJS_NewFloat64(JSContext *ctx, double val);

JSValue QJS_NewUint32(JSContext *ctx, uint32_t val);
JSValue QJS_NewUndefined(JSContext *ctx);
JSValue QJS_NewNull(JSContext *ctx);
bool QJS_IsNumber(JSContext *ctx, JSValue v);
bool QJS_IsBigInt(JSContext *ctx, JSValue v);
bool QJS_IsBool(JSContext *ctx, JSValue v);
bool QJS_IsUndefined(JSContext *ctx, JSValue v);
bool QJS_IsException(JSContext *ctx, JSValue v);
bool QJS_IsNull(JSContext *ctx, JSValue v);
bool QJS_IsString(JSContext *ctx, JSValue v);
bool QJS_IsSymbol(JSContext *ctx, JSValue v);
bool QJS_IsObject(JSContext *ctx, JSValue v);
bool QJS_IsPromise(JSContext *ctx, JSValue v);

/* Debug Only */
int QJS_GetRefCount(JSValue v);

/*
* create class
*
* @param name: Name of the JavaScript constructor function
* @param constructorCb: constructor callback function
* @param callAsFuncCb: callback function when call object as function
* #param finalizer: finalizer callback to release resource required by constructor
* @return A JSValue representing the constructor function for the class.
*
* in quickjs-ng, class ID is managed at Runtime level
* caller is responsible for avoid duplicated registration
*/
JSValue QJS_CreateClass(JSContext *ctx, const char *class_name, JSCFunction *constructorCb,
                        JSClassCall *callAsFuncCb, JSClassFinalizer *finalizer);

/*
 * create object of class represented by @param constructor
 *
 * @param: privateDate is option opaque to save into object
 */
JSValue QJS_ObjectMake(JSContext *ctx, JSValue constructor, void *privateDate);

/*
 * get private date from object
 */
void *QJS_ObjectGetPrivate(JSValue object);

#endif
