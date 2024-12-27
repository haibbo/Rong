pub use rusty_js_core::{
    function::RustFunc as CoreRustFunc, Class as CoreClass, ClassSetup, FromJSValue, IntoJSValue,
    JSClass, JSContext as CoreJSContext, JSEngine, JSException as CoreJSException,
    JSFunc as CoreJSFunc, JSObject as CoreJSObject, JSRuntime as CoreJSRuntime,
    JSValue as CoreJSValue,
};

#[cfg(feature = "quickjs")]
mod engine {
    use rusty_js_quickjs::QuickJS;
    pub type ActiveJSEngine = QuickJS;
}

#[cfg(feature = "quickjs")]
pub use engine::*;

pub type JSEngineValue = <ActiveJSEngine as JSEngine>::Value;
pub type JSEngineContext = <ActiveJSEngine as JSEngine>::Context;

pub type JSContext = CoreJSContext<<ActiveJSEngine as JSEngine>::Context>;

pub type JSValue = CoreJSValue<JSEngineValue>;
pub type JSObject = CoreJSObject<JSEngineValue>;
pub type JSFunc = CoreJSFunc<JSEngineValue>;
pub type JSException = CoreJSException<JSEngineValue>;

pub type RustFunc = CoreRustFunc<JSEngineValue>;
pub type Class = CoreClass<JSEngineValue>;

pub use rusty_js_core::function;
