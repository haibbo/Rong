mod class;
mod context;
mod runtime;
mod value;

mod jsc {
    // Native low-level bindings
    pub use rong_jscore_sys::*;

    pub(crate) trait IntoAttributes {
        fn into_attributes(self) -> u32;
    }

    impl IntoAttributes for u32 {
        fn into_attributes(self) -> u32 {
            self
        }
    }

    impl IntoAttributes for i32 {
        fn into_attributes(self) -> u32 {
            self as u32
        }
    }

    pub(crate) fn attr<T: IntoAttributes>(value: T) -> u32 {
        value.into_attributes()
    }
}

pub use context::JSCContext;
pub use runtime::{JSCRuntime, JavaScriptCore};
pub use value::JSCValue;
