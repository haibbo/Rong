#include "qjs.h"
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int length;
    int width;
} Rectangle;

void generic_finalizer(JSRuntime *rt, JSValue val) {
    void *ptr=QJS_ObjectGetPrivate(val);
    printf("Rectangle finalized.\n");
    if (ptr) {
        free(ptr);
    }
}

static JSValue rectangle_area(JSContext *ctx, JSValue this_val, int argc, JSValue *argv) {
    Rectangle *rect = QJS_ObjectGetPrivate(this_val);
    if (!rect) {
        return JS_EXCEPTION;
    }
    return JS_NewInt32(ctx, rect->length * rect->width);
}

static JSValue rectangle_get_width(JSContext *ctx, JSValue this_val, int argc, JSValue *argv) {
    Rectangle *rect = QJS_ObjectGetPrivate(this_val);
    if (!rect) {
        return JS_EXCEPTION;
    }
    return JS_NewInt32(ctx, rect->width);
}

static JSValue rectangle_set_width(JSContext *ctx, JSValue this_val, int argc, JSValue *argv) {
    Rectangle *rect = QJS_ObjectGetPrivate(this_val);
    int32_t width;
    if (!rect) {
        return JS_EXCEPTION;
    }
    if (JS_ToInt32(ctx, &width, argv[0])) {
        return JS_EXCEPTION;
    }
    rect->width = width;
    return JS_UNDEFINED;
}

static JSValue create_rectangle(JSContext *ctx, JSValue new_target, int argc, JSValue *argv) {

    if (JS_IsUndefined(new_target)) {
        printf("Rectangle called as function!\n");
        return JS_UNDEFINED;
    }

    printf("Rectangle called as constructor!\n");
        if (argc < 2 || !JS_IsNumber(argv[0]) || !JS_IsNumber(argv[1])) {
        return JS_ThrowTypeError(ctx, "Expected two numbers");
    }

    int64_t length, width;
    if (JS_ToInt64(ctx, &length, argv[0]) || JS_ToInt64(ctx, &width, argv[1])) {
        return JS_ThrowTypeError(ctx, "Invalid number");
    }

    Rectangle *rect = malloc(sizeof(Rectangle));
    if (!rect) {
        return JS_ThrowOutOfMemory(ctx);
    }
    rect->length = length;
    rect->width = width;

    return QJS_ObjectMake(ctx, new_target, rect);
}

static JSValue js_point_static_method(JSContext *ctx, JSValue this_val, int argc, JSValue *argv) {
    // Perform some static action
    printf("Static method called\n");
    return JS_UNDEFINED;
}

JSValue my_object_call(JSContext *ctx, JSValueConst func_obj, JSValueConst this_val,
                       int argc, JSValueConst *argv, int flags) {
    printf("object called as function. argc=%d\n", argc);
    return JS_UNDEFINED;
}

void setupClass(JSContext *ctx, JSValue constructor) {

    JSValue proto = JS_GetPropertyStr(ctx, constructor, "prototype");

    JS_DefinePropertyValueStr(
        ctx,
        proto,
        "area",
        JS_NewCFunction(ctx, rectangle_area, "area", 0),
        JS_PROP_WRITABLE | JS_PROP_CONFIGURABLE
    );

    JSAtom y_atom = JS_NewAtom(ctx, "width");
    JS_DefinePropertyGetSet(ctx, proto, y_atom,
                            JS_NewCFunction(ctx, rectangle_get_width, "getWidth", 0),
                            JS_NewCFunction(ctx, rectangle_set_width, "setWidth", 1),
                            JS_PROP_CONFIGURABLE);
    JS_FreeAtom(ctx, y_atom);

    // static value & method works without new instance of class
    JS_DefinePropertyValueStr(
        ctx,
        constructor,
        "staticValue",
        JS_NewInt32(ctx, 55),
        JS_PROP_WRITABLE | JS_PROP_CONFIGURABLE
    );

    // Add static method
    JS_DefinePropertyValueStr(
        ctx,
        constructor,
        "staticMethod",
        JS_NewCFunction(ctx, js_point_static_method, "staticMethod", 0),
        JS_PROP_WRITABLE | JS_PROP_CONFIGURABLE
    );

    JS_FreeValue(ctx, proto);
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

static void js_add_console(JSContext *ctx)
{
    JSValue global_obj, console ;

    global_obj = JS_GetGlobalObject(ctx);
    console = JS_NewObject(ctx);

    JS_SetPropertyStr(ctx, console, "log",
                      JS_NewCFunction(ctx, js_print, "log", 1));
    JS_SetPropertyStr(ctx, global_obj, "console", console);

    JS_FreeValue(ctx, global_obj);
    // don't free console here
}

int main(int argc, char **argv) {
    JSRuntime *rt;
    JSContext *ctx;

    rt = JS_NewRuntime();
    ctx = JS_NewContext(rt);
    js_add_console(ctx);

    JSValue constructor=QJS_CreateClass(ctx, "Rectangle", create_rectangle,NULL, generic_finalizer);
    setupClass(ctx, constructor) ;

    JSValue global_obj=JS_GetGlobalObject(ctx);
    JS_SetPropertyStr(ctx, global_obj, "Rectangle", constructor);
    JS_FreeValue(ctx,constructor);
    JS_FreeValue(ctx,global_obj);

    const char *script =
      "console.log('Rectangle: ', typeof Rectangle);\n"
      "console.log('Rectangle.constructo.name: ', Rectangle.constructor.name);\n"
      "let rect = new Rectangle(10, 18);\n"
      "console.log('rect is instanceof Rectangle: ', rect instanceof Rectangle);\n"
      "Rectangle();\n"
      "rect.width=16;\n"
      "console.log('Area: ', rect.area());\n"
      "rect=null;\n"
      "console.log('StaticValue: ', Rectangle.staticValue);\n"
      "Rectangle.staticMethod();\n";

    JSValue result=QJS_RunScript(ctx, script, strlen(script));
    if (JS_IsException(result)) {
        JSValue exception = JS_GetException(ctx);
        const char *error = JS_ToCString(ctx, exception);
        printf("Exception: %s\n", error);
        JS_FreeCString(ctx, error);
        JS_FreeValue(ctx, exception);
    }

    JS_FreeValue(ctx, result);
    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);
    return 0;
}
