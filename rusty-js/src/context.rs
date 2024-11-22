use crate::{JSCtxInner, JSRuntime, JSValue};

#[derive(Clone)]
pub struct JSCtx(pub(crate) JSCtxInner);

impl JSCtx {
    pub fn new(rt: &JSRuntime) -> Result<Self, String> {
        JSCtxInner::new(&rt.inner).map(|inner| JSCtx(inner))
    }

    // pub fn eval(ctx: &JSCtx, script: &str) -> Result<JSValue, JSValue> {}
}
