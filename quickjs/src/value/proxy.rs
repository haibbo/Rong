use crate::{QJSValue, qjs};
use rong_core::JSValueImpl;
use rong_core::engine::JSProxyOps;

impl JSProxyOps for QJSValue {
    fn new_proxy(ctx: &Self::Context, target: Self, handler: Self) -> Result<Self, Self> {
        let ctx = ctx.to_raw();
        let proxy = unsafe {
            qjs::JS_NewProxy(ctx, target.raw_value_for_api(), handler.raw_value_for_api())
        };
        if unsafe { qjs::QJS_IsException(ctx, proxy) } {
            Err(
                QJSValue::from_owned_raw(ctx, unsafe { qjs::JS_GetException(ctx) })
                    .with_exception(),
            )
        } else {
            Ok(QJSValue::from_owned_raw(ctx, proxy))
        }
    }

    fn proxy_target(&self) -> Result<Self, Self> {
        let target = unsafe { qjs::JS_GetProxyTarget(self.ctx, self.raw_value_for_api()) };
        if unsafe { qjs::QJS_IsException(self.ctx, target) } {
            Err(
                QJSValue::from_owned_raw(self.ctx, unsafe { qjs::JS_GetException(self.ctx) })
                    .with_exception(),
            )
        } else {
            Ok(QJSValue::from_owned_raw(self.ctx, target))
        }
    }
}
