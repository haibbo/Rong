#include "quickjs.h"

#include "qjs.h"

// used for getting class ID from constructor etc
extern JS_BOOL _QJS_SetCFuncMagic(JSValue func_obj, int magic);
extern int _QJS_GetCFuncMagic(JSValue func_obj);

// TODO: handle exeception
JSValue QJS_RunScript(JSContext *ctx, char *script, int len){
    JSValue val;

    val = JS_Eval(ctx, script, len, "script", 0);
    return val;
}

// TODO: handle exeception
void QJS_RunJobs(JSRuntime *rt){
    int ret;
    JSContext *ctx;

    for(;;) {

        ret=JS_ExecutePendingJob(rt, &ctx);
        if (ret==0) break; // no job pending

        if (ret<0){
           // TODO: handle exeception
            break;
        }
    }
}
