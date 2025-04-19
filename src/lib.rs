pub use rong_core::{
    Class as CoreClass, ClassSetup, FromJSValue, IntoJSAsyncIterator, IntoJSIterator, IntoJSResult,
    IntoJSValue, JSArray as CoreJSArray, JSArrayBuffer as CoreJSArrayBuffer, JSClass,
    JSContext as CoreJSContext, JSEngine, JSException as CoreJSException, JSFunc as CoreJSFunc,
    JSObject as CoreJSObject, JSResult, JSRuntime as CoreJSRuntime, JSRuntimeService,
    JSSymbol as CoreJSSymbol, JSTypedArray as CoreJSTypedArray, JSTypedArrayKind,
    JSValue as CoreJSValue, JSValueType, JsonToJsValue, Promise as CorePromise,
    PropertyDescriptor as CorePropertyDescriptor, Rong, RongJSError, Source, SourceKind,
    ToJSAsyncIterator, ToJSIterator,
};

pub use rong_core::function;

#[cfg(feature = "quickjs")]
mod engine {
    use rong_quickjs::QuickJS;
    pub type RongJS = QuickJS;
}

#[cfg(feature = "jscore")]
mod engine {
    use rong_jscore::JavaScriptCore;
    pub type RongJS = JavaScriptCore;
}

pub use engine::*;

pub type JSEngineValue = <RongJS as JSEngine>::Value;
pub type JSEngineContext = <RongJS as JSEngine>::Context;

pub type JSContext = CoreJSContext<<RongJS as JSEngine>::Context>;
pub type JSRuntime = CoreJSRuntime<<RongJS as JSEngine>::Runtime>;

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

// re-export macro public symbols to rong
pub use rong_macro::{FromJSObj, FromJSValue, js_class, js_export, js_method};

/// A Trait for conversion from JavaScript values.
pub trait TryFromJSValue: Sized {
    fn try_from_js(_value: JSValue) -> JSResult<Self>;
}
