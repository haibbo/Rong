#include "quickjs.h"
#include <uv.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

typedef struct {
    uv_timer_t handle;
    JSContext *ctx;
    JSValue resolve;
    int timeout;
} TimerData;

void on_timeout(uv_timer_t *handle) {
    TimerData *data = (TimerData *)handle->data;
    JSContext *ctx = data->ctx;

    JSValue timeout=JS_NewInt32(ctx, data->timeout);
    JS_Call(ctx, data->resolve, JS_UNDEFINED, 1, &timeout);

    JS_FreeValue(ctx, timeout);
    JS_FreeValue(ctx, data->resolve);
    free(data);
}

static JSValue js_setTimeout(JSContext *ctx, JSValueConst this_val,
                             int argc, JSValueConst *argv) {
    if (argc < 1 || !JS_IsNumber(argv[0])) {
        return JS_ThrowTypeError(ctx, "Invalid arguments: delay must be a number");
    }

    int delay;
    JS_ToInt32(ctx, &delay, argv[0]);

    JSValue resolving_funcs[2];
    JSValue promise = JS_NewPromiseCapability(ctx, resolving_funcs);
    if (JS_IsException(promise)) {
        return promise;
    }

    TimerData *data = malloc(sizeof(TimerData));
    if (!data) {
        JS_FreeValue(ctx, resolving_funcs[0]); // free resolve
        JS_FreeValue(ctx, resolving_funcs[1]); // free reject
        return JS_ThrowOutOfMemory(ctx);
    }
    data->ctx = ctx;
    data->resolve = resolving_funcs[0];
    data->timeout=delay;

    uv_timer_init(uv_default_loop(), &data->handle);
    data->handle.data = data;
    uv_timer_start(&data->handle, on_timeout, delay, 0);

    // it's useless
    JS_FreeValue(ctx, resolving_funcs[1]);

    return promise;
}

void executePendingJobs(JSRuntime *rt) {
    JSContext *ctx;
    while (JS_IsJobPending(rt)) {
        JS_ExecutePendingJob(rt, &ctx);
    }
}

static JSValue js_print(JSContext *ctx, JSValue this_val,
                        int argc, JSValue *argv)
{
    int i;
    const char *str;
    size_t len;
    for(i = 0; i < argc; i++) {
        if (i != 0)
            putchar(' ');
        str = JS_ToCStringLen(ctx, &len, argv[i]);
        if (!str)
            return JS_EXCEPTION;
        fwrite(str, 1, len, stdout);
        JS_FreeCString(ctx, str);
    }
    putchar('\n');
    fflush(stdout);
    return JS_UNDEFINED;
}

static void js_add_extensions(JSContext *ctx)
{
    JSValue global_obj, console;

    global_obj = JS_GetGlobalObject(ctx);

    console = JS_NewObject(ctx);
    JS_SetPropertyStr(ctx, console, "log",
                      JS_NewCFunction(ctx, js_print, "log", 1));
    JS_SetPropertyStr(ctx, global_obj, "console", console);

    JS_SetPropertyStr(ctx, global_obj, "print",
                      JS_NewCFunction(ctx, js_print, "print", 1));

    JS_FreeValue(ctx, global_obj);
}

int main(int argc, char **argv) {
    JSRuntime *rt = JS_NewRuntime();
    JSContext *ctx = JS_NewContext(rt);

    js_add_extensions(ctx);

    JSValue globalObj = JS_GetGlobalObject(ctx);
    JS_SetPropertyStr(ctx, globalObj, "setTimeout", JS_NewCFunction(ctx, js_setTimeout, "setTimeout", 1));
    JS_FreeValue(ctx, globalObj);

    const char *script =
        "setTimeout(2000).then((timeout) => {console.log('Elapsed time:', timeout);});";
    JSValue result = JS_Eval(ctx, script, strlen(script), "<input>", JS_EVAL_TYPE_GLOBAL);

    while (uv_loop_alive(uv_default_loop())) {
        uv_run(uv_default_loop(), UV_RUN_NOWAIT);
        executePendingJobs(rt);
    }

    JS_FreeValue(ctx, result);
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);

    return 0;
}
