#include "quickjs.h"
#include "qjs.h"

// used for set class ID to constructor
extern JS_BOOL _QJS_SetCFuncMagic(JSValue func_obj, int magic);
// used for get class ID from constructor
extern int _QJS_GetCFuncMagic(JSValue func_obj);

JSValue QJS_RunJobs(JSRuntime *rt){
    int ret;
    JSContext *ctx;

    for(;;) {

        ret=JS_ExecutePendingJob(rt, &ctx);
        if (ret==0) {
            return JS_UNDEFINED; // no job pending
        }

        if (ret<0){
            return JS_EXCEPTION;
        }
    }

    return JS_UNDEFINED;
}

JSValue QJS_CreateClass(JSContext *ctx, const char *class_name, JSCFunction *constructorCb,
                        JSClassCall *callAsFuncCb, JSClassFinalizer *finalizer) {

    JSRuntime *rt=JS_GetRuntime(ctx);

    JSClassID class_id=0; // it's important
    JS_NewClassID(rt, &class_id);

    JSClassDef class_def = {
        class_name,
        .call = callAsFuncCb,
        .finalizer = finalizer,
        .gc_mark = NULL,
        .exotic = NULL,
    };

    // register class
    JS_NewClass(rt, class_id, &class_def);

    // new prototype object
    JSValue prototype = JS_NewObject(ctx);

    // create constructor for class
    JSValue constructor= JS_NewCFunction(ctx, constructorCb, class_name, 0);
    JS_SetConstructorBit(ctx, constructor, 1);
    _QJS_SetCFuncMagic(constructor,class_id);

    // set prototype.constructor
    JS_SetConstructor(ctx, constructor, prototype);
    // set constructor.prototype
    JS_SetClassProto(ctx, class_id, prototype);

    return constructor;
}

/*
 * make object instance of class represented by Class @param constructor
 * caller needs to make sure @constructor is Constructor
 */
JSValue QJS_ObjectMake(JSContext *ctx, JSValue constructor, void *privateDate){

    int class_id = _QJS_GetCFuncMagic(constructor);
    JS_FreeValue(ctx, constructor);

    JSValue obj = JS_NewObjectClass(ctx, class_id);
    JS_SetOpaque(obj, privateDate);
    return obj;
}

// caller should make sure it's object
void *QJS_ObjectGetPrivate(JSValue object) {
    int class_id=JS_GetClassID(object);
    return JS_GetOpaque(object, class_id);
}

JS_BOOL QJS_IsPromise(JSContext *ctx, JSValue v)
{
    // JS_CLASS_PROMISE == 47
    void *p = JS_GetOpaque(v, 47);

    return p != NULL;
}
