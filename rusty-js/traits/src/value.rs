/// New JSValue from T of rust type with Context
pub trait FromWithCtx<'ctx, T> {
    type Context;
    fn from_with_ctx(ctx: &'ctx Self::Context, value: T) -> Self;
}

/// New JSValue from raw
pub trait FromRaw<'ctx, T> {
    type Context;
    fn from_raw(ctx: &'ctx Self::Context, raw: T) -> Self;
}

/// help implement traits for JSValueInner
#[macro_export]
macro_rules! impl_js_value {
    ($type:ty, $create_fn:expr, $to_fn:expr) => {
        impl<'ctx> FromWithCtx<'ctx, $type> for JSValueInner<'ctx> {
            type Context = JSCtxInner;
            fn from_with_ctx(ctx: &'ctx Self::Context, value: $type) -> Self {
                let value = unsafe { $create_fn(ctx.as_ptr(), value) };
                JSValueInner::from_raw(ctx, value)
            }
        }

        impl<'ctx> TryInto<$type> for JSValueInner<'ctx> {
            type Error = (); // don't care error detail
            fn try_into(self) -> Result<$type, Self::Error> {
                let mut result: $type = Default::default();
                if unsafe { $to_fn(self.ctx.as_ptr(), &mut result, self.value) } < 0 {
                    Err(())
                } else {
                    Ok(result)
                }
            }
        }
    };
}

/// help implement traits for JSValue
#[macro_export]
macro_rules! impl_js_values {
    ($($type:ty),*) => {
        $(
            impl<'ctx> FromWithCtx<'ctx, $type> for JSValue<'ctx> {
                type Context = JSCtx;
                fn from_with_ctx(ctx: &'ctx Self::Context, value: $type) -> Self {
                    JSValue(JSValueInner::from_with_ctx(&ctx.0, value))
                }
            }

            impl<'ctx> TryInto<$type> for JSValue<'ctx> {
                type Error=();
                fn try_into(self) -> Result<$type, Self::Error> {
                    self.0.try_into()
                }
            }
        )*
    };
}
