pub use rong_core::err_data;
pub use rong_core::{
    Class as CoreClass, ClassSetup, FromJSValue, HostError, IntoJSAsyncIteratorExt,
    IntoJSIteratorExt, IntoJSValue, JSArray as CoreJSArray, JSArrayBuffer as CoreJSArrayBuffer,
    JSAsyncIterator, JSClass, JSContext as CoreJSContext, JSContextService, JSDate as CoreJSDate,
    JSEngine, JSException as CoreJSException, JSFunc as CoreJSFunc, JSIterator,
    JSObject as CoreJSObject, JSResult, JSRuntime as CoreJSRuntime, JSRuntimeService,
    JSSymbol as CoreJSSymbol, JSTypedArray as CoreJSTypedArray, JSTypedArrayKind,
    JSValue as CoreJSValue, JSValueType, JsonToJSValue, Promise as CorePromise,
    PropertyDescriptor as CorePropertyDescriptor, RongJSError, Source, SourceKind, error,
};
// Re-export selected runtime API from rong_core::rong so downstream crates use `rong::...`
pub use rong_core::rong::{Rong, Worker, WorkerMessage, spawn};

// Re-export user-agent helpers.
pub use rong_rt::{DEFAULT_USER_AGENT, get_user_agent, set_user_agent};
// Re-export selected scheduler APIs (module remains internal to core)
pub use rong_core::{JsInvokePriority, enqueue_js_invoke};

pub mod function {
    pub use rong_core::function::{
        Constructor, FromParams, IntoJSCallable, IntoOnceJSCallable, JSParameterType, KAsyncFnMut,
        KAsyncFnOnce, KFnMut, KFnOnce, Optional, ParamsAccessor, Rest, This,
    };

    #[cfg(any(feature = "quickjs", feature = "jscore"))]
    pub type ThisMut<T> = rong_core::function::ThisMut<T, crate::JSEngineValue>;

    #[cfg(not(any(feature = "quickjs", feature = "jscore")))]
    pub type ThisMut<T> = rong_core::function::ThisMut<T, ()>;
}

#[cfg(all(feature = "quickjs", feature = "jscore"))]
compile_error!(
    "`rong` engine features are mutually exclusive: enable exactly one of `quickjs` or `jscore`."
);

#[cfg(feature = "arkjs")]
compile_error!("`arkjs` engine is not available on crates.io yet. Use `quickjs` or `jscore`.");

#[cfg(feature = "quickjs")]
mod engine {
    use rong_quickjs::QuickJS;
    pub type RongJS = QuickJS;
}

#[cfg(all(not(feature = "quickjs"), feature = "jscore"))]
mod engine {
    use rong_jscore::JavaScriptCore;
    pub type RongJS = JavaScriptCore;
}

// When no engine is selected, the engine types are not available
// This allows the crate to compile for modules that don't use the engine directly
#[cfg(all(not(feature = "quickjs"), not(feature = "jscore")))]
mod engine {}

#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub use engine::*;

#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSEngineValue = <RongJS as JSEngine>::Value;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSEngineContext = <RongJS as JSEngine>::Context;

#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSContext = CoreJSContext<<RongJS as JSEngine>::Context>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSRuntime = CoreJSRuntime<<RongJS as JSEngine>::Runtime>;

#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSValue = CoreJSValue<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSObject = CoreJSObject<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSSymbol = CoreJSSymbol<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSFunc = CoreJSFunc<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSDate = CoreJSDate<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type Class = CoreClass<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type Promise = CorePromise<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSException = CoreJSException<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSArray = CoreJSArray<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSArrayBuffer<T> = CoreJSArrayBuffer<JSEngineValue, T>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type JSTypedArray = CoreJSTypedArray<JSEngineValue>;
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub type PropertyDescriptor = CorePropertyDescriptor<JSEngineValue>;

// re-export macro public symbols to rong
pub use rong_macro::{FromJSObj, FromJSValue, IntoJSObj, js_class, js_export, js_method};

/// A Trait for conversion from JavaScript values.
#[cfg(any(feature = "quickjs", feature = "jscore"))]
pub trait TryFromJSValue: Sized {
    fn try_from_js(_value: JSValue) -> JSResult<Self>;
}
