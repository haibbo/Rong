use crate::JSCtxInner;
use crate::JSRuntime;

#[derive(Clone)]
pub struct JSCtx(pub(crate) JSCtxInner);

impl JSCtx {
    pub fn new(rt: &JSRuntime) -> Result<Self, String> {
        JSCtxInner::new(&rt.inner).map(|inner| JSCtx(inner))
    }
}
