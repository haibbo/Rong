#include "quickjs.h"

// TODO: handle exeception
JSValue QJS_RunScript(JSContext *ctx, char *script, int len){
    JSValue val;
    JSContext *ctx1;
    JSRuntime *rt;
    int ret;

    rt = JS_GetRuntime(ctx);
    val = JS_Eval(ctx, script, len, "script", 0);
    return val;
}

// TODO: handle exeception
void QJS_RunJobs(JSRuntime *rt)
    for(;;) {

        ret=JS_ExecutePendingJob(rt, &ctx1);
        if (ret==0) break; // no job pending

        if (ret<0){
           // TODO: handle exeception
            break;
        }
    }
)
