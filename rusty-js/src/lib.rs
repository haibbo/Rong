pub use rusty_js_core::{
    call, Class as CoreClass, ClassSetup, FromJSValue, IntoJSResult, IntoJSValue,
    JSArray as CoreJSArray, JSArrayBuffer as CoreJSArrayBuffer, JSClass,
    JSContext as CoreJSContext, JSEngine, JSException as CoreJSException, JSFunc as CoreJSFunc,
    JSObject as CoreJSObject, JSResult, JSRuntime as CoreJSRuntime, JSRuntimeService,
    JSSymbol as CoreJSSymbol, JSTypedArray as CoreJSTypedArray, JSTypedArrayKind,
    JSValue as CoreJSValue, JSValueType, JsonToJsValue, Promise as CorePromise,
    PropertyDescriptor as CorePropertyDescriptor, RustyJSError, Source,
};

pub use rusty_js_core::function;

#[cfg(feature = "quickjs")]
mod engine {
    use rusty_js_quickjs::QuickJS;
    pub type RustyJS = QuickJS;
}

#[cfg(feature = "jscore")]
mod engine {
    use rusty_js_jscore::JavaScriptCore;
    pub type RustyJS = JavaScriptCore;
}

pub use engine::*;

pub type JSEngineValue = <RustyJS as JSEngine>::Value;
pub type JSEngineContext = <RustyJS as JSEngine>::Context;

pub type JSContext = CoreJSContext<<RustyJS as JSEngine>::Context>;
pub type JSRuntime = CoreJSRuntime<<RustyJS as JSEngine>::Runtime>;

pub type JSValue = CoreJSValue<JSEngineValue>;
pub type JSObject = CoreJSObject<JSEngineValue>;
pub type JSSymbol = CoreJSSymbol<JSEngineValue>;
pub type JSFunc = CoreJSFunc<JSEngineValue>;
pub type Class = CoreClass<JSEngineValue>;
pub type Promise = CorePromise<JSEngineValue>;
pub type JSException = CoreJSException<JSEngineValue>;
pub type JSArray = CoreJSArray<JSEngineValue>;
pub type JSArrayBuffer<T> = CoreJSArrayBuffer<JSEngineValue, T>;
pub type JSTypedArray = CoreJSTypedArray<JSEngineValue>;
pub type PropertyDescriptor = CorePropertyDescriptor<JSEngineValue>;

// re-export macro public symbols to rusty_js
pub use rusty_js_macro::{js_class, js_method, js_methods};
