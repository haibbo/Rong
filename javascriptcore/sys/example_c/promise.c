#include <JavaScriptCore/JavaScript.h>
#include <uv.h>
#include <stdio.h>

// Custom print function for outputting information in JavaScript
JSValueRef PrintCallback(JSContextRef ctx, JSObjectRef function, JSObjectRef thisObject,
                         size_t argumentCount, const JSValueRef arguments[], JSValueRef* exception) {
    for (size_t i = 0; i < argumentCount; i++) {
        JSStringRef string = JSValueToStringCopy(ctx, arguments[i], exception);
        if (!string) {
            fprintf(stderr, "Error converting argument to string\n");
            return JSValueMakeUndefined(ctx);
        }
        size_t bufferSize = JSStringGetMaximumUTF8CStringSize(string);
        char* buffer = (char*)malloc(bufferSize);
        JSStringGetUTF8CString(string, buffer, bufferSize);
        printf("%s ", buffer);
        free(buffer);
        JSStringRelease(string);
    }
    printf("\n");
    return JSValueMakeUndefined(ctx);
}

// Register global print function
void RegisterPrintFunction(JSContextRef ctx, JSObjectRef globalObject) {
    JSStringRef printName = JSStringCreateWithUTF8CString("print");
    JSObjectRef printFunction = JSObjectMakeFunctionWithCallback(ctx, printName, PrintCallback);
    JSObjectSetProperty(ctx, globalObject, printName, printFunction, kJSPropertyAttributeNone, NULL);
    JSStringRelease(printName);
}

void SetTimeoutCallback(uv_timer_t* handle) {
    JSContextRef ctx = (JSContextRef)handle->data;
    JSValueRef callback = (JSValueRef)handle->loop->data;
    JSObjectCallAsFunction(ctx, (JSObjectRef)callback, NULL, 0, NULL, NULL);
    uv_close((uv_handle_t*)handle, NULL);
}

JSValueRef SetTimeout(JSContextRef ctx, JSObjectRef function, JSObjectRef thisObject,
                      size_t argumentCount, const JSValueRef arguments[], JSValueRef* exception) {
    if (argumentCount < 2 || !JSValueIsObject(ctx, arguments[0]) || !JSValueIsNumber(ctx, arguments[1])) {
        *exception = JSValueMakeString(ctx, JSStringCreateWithUTF8CString("Invalid arguments"));
        return JSValueMakeUndefined(ctx);
    }

    // Get callback function and delay time
    JSValueRef callback = arguments[0];
    int delay = JSValueToNumber(ctx, arguments[1], NULL);

    // Create libuv timer
    uv_timer_t* timer = (uv_timer_t*)malloc(sizeof(uv_timer_t));
    uv_timer_init(uv_default_loop(), timer);
    timer->data = (void*)ctx;
    uv_timer_start(timer, SetTimeoutCallback, delay, 0);

    // Store callback function in event loop
    uv_default_loop()->data = (void*)callback;

    return JSValueMakeUndefined(ctx);
}

// Register global setTimeout function
void RegisterSetTimeoutFunction(JSContextRef ctx, JSObjectRef globalObject) {
    JSStringRef setTimeoutName = JSStringCreateWithUTF8CString("setTimeout");
    JSObjectRef setTimeoutFunction = JSObjectMakeFunctionWithCallback(ctx, setTimeoutName, SetTimeout);
    JSObjectSetProperty(ctx, globalObject, setTimeoutName, setTimeoutFunction, kJSPropertyAttributeNone, NULL);
    JSStringRelease(setTimeoutName);
}

// C callback function for handling then event
JSValueRef ThenCallback(JSContextRef ctx, JSObjectRef function, JSObjectRef thisObject,
                        size_t argumentCount, const JSValueRef arguments[], JSValueRef* exception) {
    if (argumentCount > 0 && !JSValueIsUndefined(ctx, arguments[0])) {
        JSStringRef valueString = JSValueToStringCopy(ctx, arguments[0], exception);
        if (valueString) {
            size_t bufferSize = JSStringGetMaximumUTF8CStringSize(valueString);
            char* buffer = (char*)malloc(bufferSize);
            JSStringGetUTF8CString(valueString, buffer, bufferSize);
            printf("Then callback in C: %s\n", buffer);
            free(buffer);
            JSStringRelease(valueString);
        }
    } else {
        printf("Then callback in C: No value received\n");
    }
    return JSValueMakeUndefined(ctx);
}

// C callback function for handling catch event
JSValueRef CatchCallback(JSContextRef ctx, JSObjectRef function, JSObjectRef thisObject,
                         size_t argumentCount, const JSValueRef arguments[], JSValueRef* exception) {
    if (argumentCount > 0 && !JSValueIsUndefined(ctx, arguments[0])) {
        JSStringRef errorString = JSValueToStringCopy(ctx, arguments[0], exception);
        if (errorString) {
            size_t bufferSize = JSStringGetMaximumUTF8CStringSize(errorString);
            char* buffer = (char*)malloc(bufferSize);
            JSStringGetUTF8CString(errorString, buffer, bufferSize);
            printf("Catch callback in C: %s\n", buffer);
            free(buffer);
            JSStringRelease(errorString);
        }
    } else {
        printf("Catch callback in C: No error received\n");
    }
    return JSValueMakeUndefined(ctx);
}

// Bind then and catch callbacks to Promise
void BindPromiseCallbacks(JSContextRef ctx, JSValueRef promiseValue) {
    JSObjectRef promiseObject = (JSObjectRef)promiseValue;

    // Get then and catch methods
    JSStringRef thenName = JSStringCreateWithUTF8CString("then");
    JSStringRef catchName = JSStringCreateWithUTF8CString("catch");

    JSValueRef thenValue = JSObjectGetProperty(ctx, promiseObject, thenName, NULL);
    JSValueRef catchValue = JSObjectGetProperty(ctx, promiseObject, catchName, NULL);

    // Create C callback functions
    JSObjectRef thenCallback = JSObjectMakeFunctionWithCallback(ctx, NULL, ThenCallback);
    JSObjectRef catchCallback = JSObjectMakeFunctionWithCallback(ctx, NULL, CatchCallback);

    // Call then and catch methods
    JSValueRef thenCallbackValue = (JSValueRef)thenCallback;
    JSValueRef catchCallbackValue = (JSValueRef)catchCallback;
    JSObjectCallAsFunction(ctx, (JSObjectRef)thenValue, promiseObject, 1, &thenCallbackValue, NULL);
    JSObjectCallAsFunction(ctx, (JSObjectRef)catchValue, promiseObject, 1, &catchCallbackValue, NULL);

    JSStringRelease(thenName);
    JSStringRelease(catchName);
}

int main() {
    JSGlobalContextRef ctx = JSGlobalContextCreate(NULL);

    JSObjectRef globalObject = JSContextGetGlobalObject(ctx);
    RegisterPrintFunction(ctx, globalObject);
    RegisterSetTimeoutFunction(ctx, globalObject);

    // Execute JavaScript code from string
    const char* jsCode =
        "let promise = new Promise((resolve, reject) => {\n"
        "    setTimeout(() => {\n"
        "        if (Math.random() > 0.5) {\n"
        "            resolve(\"Success!\");\n"
        "        } else {\n"
        "            reject(\"Failure!\");\n"
        "        }\n"
        "    }, 1000);\n"
        "});\n"
        "\n"
        "promise.then((value) => {\n"
        "    print(\"Then callback:\", value);\n"
        "    return value;\n"
        "}).catch((error) => {\n"
        "    print(\"Catch callback:\", error);\n"
        "    return error;\n"
        "});";

    JSStringRef script = JSStringCreateWithUTF8CString(jsCode);
    JSValueRef exception = NULL;
    JSValueRef result = JSEvaluateScript(ctx, script, NULL, NULL, 1, &exception);
    if (exception) {
        JSStringRef exceptionString = JSValueToStringCopy(ctx, exception, NULL);
        size_t bufferSize = JSStringGetMaximumUTF8CStringSize(exceptionString);
        char* exceptionBuffer = (char*)malloc(bufferSize);
        JSStringGetUTF8CString(exceptionString, exceptionBuffer, bufferSize);
        fprintf(stderr, "Exception: %s\n", exceptionBuffer);
        free(exceptionBuffer);
        JSStringRelease(exceptionString);
    }

    if (JSValueIsObject(ctx, result)) {
        BindPromiseCallbacks(ctx, result);
    }

    uv_run(uv_default_loop(), UV_RUN_DEFAULT);

    // Clean up resources
    JSStringRelease(script);
    JSGlobalContextRelease(ctx);

    return 0;
}

// clang promise.c -framework JavaScriptCore -luv -o promise
