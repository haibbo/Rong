/// Convert JSValue to rust type
pub trait IntoHost<T> {
    fn into_host(self) -> Option<T>;
}

/// New JSValue from rust type
pub trait FromHost<'ctx, T>: Sized {
    type Context;
    fn from_host(ctx: &'ctx Self::Context, v: T) -> Self;
}

/// New JSValue from raw
pub trait FromRaw<'ctx, T>: Sized {
    type Context;
    fn from_raw(ctx: &'ctx Self::Context, v: T) -> Self;
}

//
/// help implement traits for JSValueInner
#[macro_export]
macro_rules! impl_js_value {
    ($type:ty, $create_fn:expr, $to_fn:expr) => {
        impl_js_value!($type, $create_fn, $to_fn, $type);
    };

    ($type:ty, $create_fn:expr, $to_fn:expr, $to_type:ty) => {
        impl<'ctx> FromHost<'ctx, $type> for JSValueInner<'ctx> {
            type Context = JSCtxInner;
            fn from_host(ctx: &'ctx Self::Context, value: $type) -> Self {
                let value = unsafe { $create_fn(ctx.as_ptr(), value) };
                JSValueInner::from_raw(ctx, value)
            }
        }

        impl<'ctx> IntoHost<$to_type> for JSValueInner<'ctx> {
            fn into_host(self) -> Option<$to_type> {
                let mut result: $to_type = Default::default();
                if unsafe { $to_fn(self.ctx.as_ptr(), &mut result, self.value) } < 0 {
                    None
                } else {
                    Some(result)
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
            impl<'ctx> FromHost<'ctx, $type> for JSValue<'ctx> {
                type Context = JSCtx;
                fn from_host(ctx: &'ctx Self::Context, value: $type) -> Self {
                    JSValue(JSValueInner::from_host(&ctx.0, value))
                }
            }

            impl<'ctx> IntoHost<$type> for JSValue<'ctx> {
                fn into_host(self) -> Option<$type> {
                    self.0.into_host()
                }
            }
        )*
    };
}
