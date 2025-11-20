pub use rong_core::{
    Class as CoreClass, ClassSetup, FromJSValue, IntoJSAsyncIteratorExt, IntoJSIteratorExt,
    IntoJSResult, IntoJSValue, JSArray as CoreJSArray, JSArrayBuffer as CoreJSArrayBuffer,
    JSAsyncIterator, JSClass, JSContext as CoreJSContext, JSContextService, JSDate as CoreJSDate,
    JSEngine, JSException as CoreJSException, JSFunc as CoreJSFunc, JSIterator,
    JSObject as CoreJSObject, JSResult, JSRuntime as CoreJSRuntime, JSRuntimeService,
    JSSymbol as CoreJSSymbol, JSTypedArray as CoreJSTypedArray, JSTypedArrayKind,
    JSValue as CoreJSValue, JSValueType, JsonToJsValue, Promise as CorePromise,
    PropertyDescriptor as CorePropertyDescriptor, RongJSError, Source, SourceKind,
};
// Re-export selected runtime API from rong_core::rong so downstream crates use `rong::...`
pub use rong_core::rong::{Rong, Worker, WorkerMessage, spawn};

// Re-export service executor APIs
pub use rong_core::service_executor::{self, get_user_agent, set_user_agent};
// Re-export selected scheduler APIs (module remains internal to core)
pub use rong_core::{JsInvokePriority, enqueue_js_invoke};

pub use rong_core::function;

#[cfg(feature = "quickjs")]
mod engine {
    use rong_quickjs::QuickJS;
    pub type RongJS = QuickJS;
}

#[cfg(all(feature = "jscore", not(feature = "quickjs"), not(feature = "arkjs")))]
mod engine {
    use rong_jscore::JavaScriptCore;
    pub type RongJS = JavaScriptCore;
}

#[cfg(all(feature = "arkjs", not(feature = "quickjs"), not(feature = "jscore")))]
mod engine {
    use rong_arkjs::HarmonyArkJS;
    pub type RongJS = HarmonyArkJS;
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
pub type JSDate = CoreJSDate<JSEngineValue>;
pub type Class = CoreClass<JSEngineValue>;
pub type Promise = CorePromise<JSEngineValue>;
pub type JSException = CoreJSException<JSEngineValue>;
pub type JSArray = CoreJSArray<JSEngineValue>;
pub type JSArrayBuffer<T> = CoreJSArrayBuffer<JSEngineValue, T>;
pub type JSTypedArray = CoreJSTypedArray<JSEngineValue>;
pub type PropertyDescriptor = CorePropertyDescriptor<JSEngineValue>;

// re-export macro public symbols to rong
pub use rong_macro::{FromJSObj, FromJSValue, IntoJSObj, js_class, js_export, js_method};

/// A Trait for conversion from JavaScript values.
pub trait TryFromJSValue: Sized {
    fn try_from_js(_value: JSValue) -> JSResult<Self>;
}
