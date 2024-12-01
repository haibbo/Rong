#include <JavaScriptCore/JavaScript.h>
#include <uv.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// 全局上下文引用
static JSGlobalContextRef globalCtx = NULL;

// 用于传递给 libuv 的数据结构
typedef struct {
    char* filepath;
    char* content;
    size_t size;
    JSObjectRef resolve;
    JSObjectRef reject;
    JSContextRef ctx;
} ReadFileData;

// print 实现
static JSValueRef jsprint(JSContextRef ctx,
                         JSObjectRef function,
                         JSObjectRef thisObject,
                         size_t argumentCount,
                         const JSValueRef arguments[],
                         JSValueRef* exception) {
    for (size_t i = 0; i < argumentCount; i++) {
        if (i > 0) printf(" ");

        JSStringRef strRef = JSValueToStringCopy(ctx, arguments[i], exception);
        size_t bufferSize = JSStringGetMaximumUTF8CStringSize(strRef);
        char* str = (char*)malloc(bufferSize);
        JSStringGetUTF8CString(strRef, str, bufferSize);
        printf("%s", str);
        free(str);
        JSStringRelease(strRef);
    }
    printf("\n");
    return JSValueMakeUndefined(ctx);
}

// libuv 文件读取回调
static void on_read(uv_fs_t* req) {
    ReadFileData* data = (ReadFileData*)req->data;

    if (req->result < 0) {
        // 读取失败
        JSStringRef errorMsg = JSStringCreateWithUTF8CString(uv_strerror(req->result));
        JSValueRef error = JSValueMakeString(data->ctx, errorMsg);
        JSValueRef args[] = { error };
        JSObjectCallAsFunction(data->ctx, data->reject, NULL, 1, args, NULL);
        JSStringRelease(errorMsg);
    } else {
        // 读取成功
        JSStringRef content = JSStringCreateWithUTF8CString(data->content);
        JSValueRef result = JSValueMakeString(data->ctx, content);
        JSValueRef args[] = { result };
        JSObjectCallAsFunction(data->ctx, data->resolve, NULL, 1, args, NULL);
        JSStringRelease(content);
    }

    // 清理资源
    free(data->content);
    free(data->filepath);
    free(data);
    uv_fs_req_cleanup(req);
    free(req);
}

// libuv 文件打开回调
static void on_open(uv_fs_t* req) {
    if (req->result < 0) {
        ReadFileData* data = (ReadFileData*)req->data;
        JSStringRef errorMsg = JSStringCreateWithUTF8CString(uv_strerror(req->result));
        JSValueRef error = JSValueMakeString(data->ctx, errorMsg);
        JSValueRef args[] = { error };
        JSObjectCallAsFunction(data->ctx, data->reject, NULL, 1, args, NULL);
        JSStringRelease(errorMsg);

        free(data->filepath);
        free(data);
        uv_fs_req_cleanup(req);
        free(req);
        return;
    }

    ReadFileData* data = (ReadFileData*)req->data;
    uv_fs_t* read_req = (uv_fs_t*)malloc(sizeof(uv_fs_t));
    read_req->data = data;

    // 获取文件大小
    uv_fs_t stat_req;
    uv_fs_fstat(uv_default_loop(), &stat_req, req->result, NULL);
    data->size = stat_req.statbuf.st_size;
    data->content = (char*)malloc(data->size + 1);
    data->content[data->size] = '\0';

    // 异步读取文件
    uv_buf_t buf = uv_buf_init(data->content, data->size);
    uv_fs_read(uv_default_loop(), read_req, req->result, &buf, 1, 0, on_read);

    uv_fs_req_cleanup(&stat_req);
    uv_fs_req_cleanup(req);
    free(req);
}

// readFile 实现
static JSValueRef readFile(JSContextRef ctx,
                          JSObjectRef function,
                          JSObjectRef thisObject,
                          size_t argumentCount,
                          const JSValueRef arguments[],
                          JSValueRef* exception) {
    if (argumentCount < 1) {
        JSStringRef errorMsg = JSStringCreateWithUTF8CString("File path required");
        *exception = JSValueMakeString(ctx, errorMsg);
        JSStringRelease(errorMsg);
        return JSValueMakeUndefined(ctx);
    }

    // 获取文件路径参数
    JSStringRef pathStr = JSValueToStringCopy(ctx, arguments[0], exception);
    size_t bufferSize = JSStringGetMaximumUTF8CStringSize(pathStr);
    char* filepath = (char*)malloc(bufferSize);
    JSStringGetUTF8CString(pathStr, filepath, bufferSize);
    JSStringRelease(pathStr);

    // 创建延迟的 Promise
    JSObjectRef resolve, reject;
    JSObjectRef promise = JSObjectMakeDeferredPromise(ctx, &resolve, &reject, exception);
    if (*exception) {
        free(filepath);
        return JSValueMakeUndefined(ctx);
    }

    // 准备异步读取数据
    ReadFileData* data = (ReadFileData*)malloc(sizeof(ReadFileData));
    data->filepath = filepath;
    data->content = NULL;
    data->ctx = ctx;
    data->resolve = resolve;
    data->reject = reject;

    // 开始异步文件操作
    uv_fs_t* open_req = (uv_fs_t*)malloc(sizeof(uv_fs_t));
    open_req->data = data;
    uv_fs_open(uv_default_loop(), open_req, filepath, O_RDONLY, 0, on_open);

    return promise;
}

// 设置全局函数
static void setupGlobalFunctions(JSContextRef ctx, JSObjectRef globalObject) {
    // 设置 print 函数
    JSStringRef printName = JSStringCreateWithUTF8CString("jsprint");
    JSObjectRef printFunc = JSObjectMakeFunctionWithCallback(ctx, printName, jsprint);
    JSObjectSetProperty(ctx, globalObject, printName, printFunc,
                       kJSPropertyAttributeNone, NULL);
    JSStringRelease(printName);

    // 设置 readFile 函数
    JSStringRef readFileName = JSStringCreateWithUTF8CString("readFile");
    JSObjectRef readFileFunc = JSObjectMakeFunctionWithCallback(ctx, readFileName, readFile);
    JSObjectSetProperty(ctx, globalObject, readFileName, readFileFunc,
                       kJSPropertyAttributeNone, NULL);
    JSStringRelease(readFileName);
}

int main() {
    // 初始化 JS 环境
    JSContextGroupRef group = JSContextGroupCreate();
    globalCtx = JSGlobalContextCreateInGroup(group, NULL);
    JSObjectRef globalObject = JSContextGetGlobalObject(globalCtx);

    // 设置全局函数
    setupGlobalFunctions(globalCtx, globalObject);

    // 测试脚本
    const char* script =
        "jsprint(typeof readFile('1.txt'));\n"
        "async function test() {\n"
        "    try {\n"
        "        jsprint('Reading file...');\n"
        "        const content = await readFile('test.txt');\n"
        "        jsprint('File content:', content);\n"
        "    } catch (err) {\n"
        "        jsprint('Error:', err);\n"
        "    }\n"
        "}\n"
        "test();\n";

    JSStringRef scriptJS = JSStringCreateWithUTF8CString(script);
    JSValueRef exception = NULL;
    JSEvaluateScript(globalCtx, scriptJS, NULL, NULL, 1, &exception);

    if (exception) {
        JSStringRef exceptionStr = JSValueToStringCopy(globalCtx, exception, NULL);
        size_t bufferSize = JSStringGetMaximumUTF8CStringSize(exceptionStr);
        char* str = (char*)malloc(bufferSize);
        JSStringGetUTF8CString(exceptionStr, str, bufferSize);
        printf("Exception: %s\n", str);
        free(str);
        JSStringRelease(exceptionStr);
    }

    // 运行事件循环
    uv_run(uv_default_loop(), UV_RUN_DEFAULT);

    // 清理资源
    JSStringRelease(scriptJS);
    JSGlobalContextRelease(globalCtx);
    JSContextGroupRelease(group);
    uv_loop_close(uv_default_loop());

    return 0;
}
// clang -framework JavaScriptCore -luv file_reader.c -o file_reader
