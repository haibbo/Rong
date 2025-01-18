use crate::{jsc, JSCContext};
use rusty_js_core::JSValueImpl;

mod object;

#[derive(Clone)]
pub struct JSCValue {
    value: jsc::JSValueRef,
    ctx: jsc::JSGlobalContextRef,
}

impl JSValueImpl for JSCValue {
    type RawValue = jsc::JSValueRef;
    type Context = JSCContext;

    fn from_borrowed_raw(
        ctx: <Self::Context as rusty_js_core::JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        todo!()
    }

    fn from_owned_raw(
        ctx: <Self::Context as rusty_js_core::JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        todo!()
    }

    fn into_raw_value(self) -> Self::RawValue {
        todo!()
    }

    fn as_raw_value(&self) -> &Self::RawValue {
        todo!()
    }

    fn as_raw_context(&self) -> &<Self::Context as rusty_js_core::JSContextImpl>::RawContext {
        todo!()
    }
}
