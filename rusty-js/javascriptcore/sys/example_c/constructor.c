#include <JavaScriptCore/JavaScriptCore.h>
#include <stdio.h>

//#define FORCE_SET_PROTOTYPE_TO_CONSTUCTOR

// 定义一个构造函数
static JSObjectRef RectangleConstructor(JSContextRef ctx, JSObjectRef constructor, size_t argumentCount, const JSValueRef arguments[], JSValueRef* exception) {
    // 创建一个空的对象作为实例
    printf("I'm RectangleConstructor\n");

    JSClassRef rectangleClass = JSObjectGetPrivate(constructor);
    JSObjectRef instance = JSObjectMake(ctx, rectangleClass, NULL);
    // JSObjectSetPrivate(instance, customDate);

    // Set the prototype for the new object
    #ifdef FORCE_SET_PROTOTYPE_TO_CONSTUCTOR
    // 在这个特定实现中，构造函数的原型已经被预先设置为正确的原型对象
    JSValueRef prototypeValue = JSObjectGetPrototype(ctx, constructor);
    #else  // better
    JSStringRef prototypeStr = JSStringCreateWithUTF8CString("prototype");
    JSValueRef prototypeValue = JSObjectGetProperty(ctx, constructor, prototypeStr, exception);
    JSStringRelease(prototypeStr);
    #endif

    JSObjectRef prototypeObject = JSValueToObject(ctx, prototypeValue, exception);
    if (prototypeObject && !*exception) {
        JSObjectSetPrototype(ctx, instance, prototypeObject);
    }

    return instance;
}

static JSValueRef RectangleCallAsFunction(JSContextRef ctx, JSObjectRef function, JSObjectRef thisObject, size_t argumentCount, const JSValueRef arguments[], JSValueRef* exception) {

    // in order to make typeof Rectangle is function, it seems we needs to use callAsFunction
    // but it indeeds do not support new, here error should be throwed.
    // TypeError: Class constructor Rectangle cannot be invoked without 'new'

    // 获取 name 属性
    JSStringRef propertyName = JSStringCreateWithUTF8CString("name");
    JSValueRef propertyValue = JSObjectGetProperty(ctx, function, propertyName, NULL);
    JSStringRelease(propertyName);

    if (JSValueIsString(ctx, propertyValue)) { // Rectangle()
        JSStringRef resultString = JSValueToStringCopy(ctx, propertyValue, NULL);
        size_t resultSize = JSStringGetMaximumUTF8CStringSize(resultString);
        char resultCString[resultSize];
        JSStringGetUTF8CString(resultString, resultCString, resultSize);
        JSStringRelease(resultString);
        printf("CallAsFunc directly on class constructor %s\n", resultCString);
    }
    else{ // new Rectangle().()

        printf("CallAsFunc on object of class\n");
    }

    // Return the current date as a string
    time_t t = time(NULL);
    struct tm *timeinfo = localtime(&t);
    char buffer[80];
    strftime(buffer, sizeof(buffer), "%c", timeinfo);

    JSStringRef resultStr = JSStringCreateWithUTF8CString(buffer);
    JSValueRef result = JSValueMakeString(ctx, resultStr);
    JSStringRelease(resultStr);

    return result;
}

// Implementation of the Date.prototype.toString method
static JSValueRef DateToString(JSContextRef ctx, JSObjectRef function, JSObjectRef thisObject, size_t argumentCount, const JSValueRef arguments[], JSValueRef *exception) {

    time_t timestamp;
    struct tm *timeinfo = localtime(&timestamp);
    char buffer[80];
	  strftime(buffer, sizeof(buffer), "%Y-%m-%dT%H:%M:%SZ", timeinfo);

	  JSStringRef resultRef = JSStringCreateWithUTF8CString(buffer);
	  JSValueRef result = JSValueMakeString(ctx, resultRef);
	  JSStringRelease(resultRef);

	  return result;
    // return JSValueMakeUndefined(ctx);
}

int main() {
    // 创建全局执行上下文
    JSGlobalContextRef ctx = JSGlobalContextCreate(NULL);

    // 创建构造函数
    JSClassDefinition classDef = kJSClassDefinitionEmpty;
    classDef.className = "Rectangle";
    classDef.callAsConstructor = RectangleConstructor;  // callAsx must cowork with JSObjectMake not JSObjectMakeConstructor
    classDef.callAsFunction = RectangleCallAsFunction; // typeof Rectangle is function
    JSClassRef rectangleClass = JSClassCreate(&classDef);


    // 构造函数
    // JSObjectRef rectangleConstructor = JSObjectMakeConstructor(ctx, rectangleClass, RectangleConstructor);
    JSObjectRef rectangleConstructor= JSObjectMake(ctx, rectangleClass, NULL);

    // for constructor function to get classDef
    JSObjectSetPrivate(rectangleConstructor, rectangleClass);

   // 设置构造函数的 name 属性
    JSStringRef nameProperty = JSStringCreateWithUTF8CString("name");
    JSStringRef nameValue = JSStringCreateWithUTF8CString("Rectangle");
    JSValueRef nameValueRef = JSValueMakeString(ctx, nameValue);
    JSObjectSetProperty(ctx, rectangleConstructor, nameProperty, nameValueRef, kJSPropertyAttributeReadOnly | kJSPropertyAttributeDontEnum, NULL);
    JSStringRelease(nameProperty);
    JSStringRelease(nameValue);

    // 设置构造函数的 length 属性, jscore 根本不会 check 构造函数输入参数是否满足
    JSStringRef lengthProp = JSStringCreateWithUTF8CString("length"); // the number of arguments of constructor
    JSValueRef lengthValue = JSValueMakeNumber(ctx, 2);
    JSObjectSetProperty(ctx, rectangleConstructor, lengthProp, lengthValue, kJSPropertyAttributeReadOnly | kJSPropertyAttributeDontEnum | kJSPropertyAttributeDontDelete, NULL);
    JSStringRelease(lengthProp);



    // 获取全局对象
    JSObjectRef globalObject = JSContextGetGlobalObject(ctx);

    // 获取 Function 构造器
    JSStringRef functionName = JSStringCreateWithUTF8CString("Function");
    JSValueRef functionValue = JSObjectGetProperty(ctx, globalObject, functionName, NULL);
    JSStringRelease(functionName);

    // 确保 functionValue 是一个对象并且是 Function
    if (JSValueIsObject(ctx, functionValue)) {
        JSObjectRef functionConstructor = JSValueToObject(ctx, functionValue, NULL);

        // 手动设置 Rectangle.constructor 为 Function
        JSStringRef constructorPropertyName = JSStringCreateWithUTF8CString("constructor");
        JSObjectSetProperty(ctx, rectangleConstructor, constructorPropertyName, functionConstructor, kJSPropertyAttributeDontEnum, NULL);
        JSStringRelease(constructorPropertyName);
    }

    // 将构造函数添加到全局对象
    JSStringRef rectangleName = JSStringCreateWithUTF8CString("Rectangle");
    JSObjectSetProperty(ctx, globalObject, rectangleName, rectangleConstructor, kJSPropertyAttributeNone, NULL);

  #if 0 // cowork with JSObjectMakeConstructor
    // 自动创建的 prototype 对象
    JSStringRef prototypePropertyName = JSStringCreateWithUTF8CString("prototype");
    JSValueRef prototypeValue = JSObjectGetProperty(ctx, rectangleConstructor, prototypePropertyName, NULL);
    if (JSValueIsObject(ctx, prototypeValue)) {
        JSObjectRef prototypeObject = JSValueToObject(ctx, prototypeValue, NULL);

        // 设置 prototype.constructor 为 rectangleConstructor
        JSStringRef constructorPropertyName = JSStringCreateWithUTF8CString("constructor");
        JSObjectSetProperty(ctx, prototypeObject, constructorPropertyName, rectangleConstructor, kJSPropertyAttributeDontEnum, NULL);
        JSStringRelease(constructorPropertyName);
    }
    JSStringRelease(prototypePropertyName);
  #else
    // 定义 prototype 对象
    JSObjectRef prototypeObject = JSObjectMake(ctx, NULL, NULL);
    JSStringRef prototypeProp = JSStringCreateWithUTF8CString("prototype");
    JSObjectSetProperty(ctx, rectangleConstructor, prototypeProp, prototypeObject, kJSPropertyAttributeDontEnum | kJSPropertyAttributeReadOnly | kJSPropertyAttributeDontDelete, NULL);
    JSStringRelease(prototypeProp);

    #ifdef FORCE_SET_PROTOTYPE_TO_CONSTUCTOR
    JSObjectSetPrototype(ctx, rectangleConstructor, prototypeObject);
    #endif

    // 设置 prototype.constructor 为 rectangleConstructor
    JSStringRef constructorPropertyName = JSStringCreateWithUTF8CString("constructor");
    JSObjectSetProperty(ctx, prototypeObject, constructorPropertyName, rectangleConstructor, kJSPropertyAttributeDontEnum, NULL);
    JSStringRelease(constructorPropertyName);
  #endif

    // Set Date.prototype.toString
    JSStringRef toStringName = JSStringCreateWithUTF8CString("toString");
    JSObjectSetProperty(ctx, prototypeObject, toStringName,
                        JSObjectMakeFunctionWithCallback(ctx, toStringName, DateToString),
                        kJSPropertyAttributeNone, NULL);
    JSStringRelease(toStringName);



    // JavaScript 脚本来检查构造函数的类型
    // const char* script = "new Rectangle(2,3);Rectangle(3,4); typeof Rectangle + ', '+Rectangle.prototype.constructor.name + ', ' + Rectangle.prototype.constructor.length + ', ' + Rectangle.constructor.name + ', ' + Rectangle.constructor.length;";

    const char *script="new Rectangle(2,3).toString();";
    // const char *script="Rectangle();new Rectangle(2,3)();";

    // 执行脚本
    JSStringRef scriptJS = JSStringCreateWithUTF8CString(script);
    JSValueRef exception = NULL;
    JSValueRef result = JSEvaluateScript(ctx, scriptJS, NULL, NULL, 0, &exception);
    JSStringRelease(scriptJS);

    // 处理结果
    if (exception) {
        JSStringRef exceptionString = JSValueToStringCopy(ctx, exception, NULL);
        size_t exceptionSize = JSStringGetMaximumUTF8CStringSize(exceptionString);
        char exceptionCString[exceptionSize];
        JSStringGetUTF8CString(exceptionString, exceptionCString, exceptionSize);
        printf("Exception: %s\n", exceptionCString);
        JSStringRelease(exceptionString);
    } else {
        JSStringRef resultString = JSValueToStringCopy(ctx, result, NULL);
        size_t resultSize = JSStringGetMaximumUTF8CStringSize(resultString);
        char resultCString[resultSize];
        JSStringGetUTF8CString(resultString, resultCString, resultSize);
        printf("Result: %s\n", resultCString);
        JSStringRelease(resultString);
    }

    // 释放
    JSStringRelease(rectangleName);
    JSGlobalContextRelease(ctx);
    return 0;
}

 // clang constructor.c -framework JavaScriptCore -o constructor && ./constructor
