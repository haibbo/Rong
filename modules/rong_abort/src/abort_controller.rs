use crate::AbortSignal;
use rong::{function::*, *};

#[js_export]
pub struct AbortController {
    abort_signal: JSObject, // AbortSignal
}

#[js_class]
impl AbortController {
    #[js_method(constructor)]
    fn new(ctx: JSContext) -> JSResult<Self> {
        Ok(Self {
            abort_signal: Class::get::<AbortSignal>(&ctx)?.instance(AbortSignal::new(&ctx)),
        })
    }

    #[js_method(getter)]
    pub fn signal(&self) -> JSObject {
        self.abort_signal.clone()
    }

    #[js_method]
    pub fn abort(&self, ctx: JSContext, reason: Optional<JSValue>) -> JSResult<()> {
        let abort = self.abort_signal.borrow_mut::<AbortSignal>()?;
        if abort.aborted() {
            //only once
            return Ok(());
        }
        abort.set_reason(reason);
        drop(abort);
        AbortSignal::broadcast_abort(&ctx, This(self.abort_signal.clone()))?;
        Ok(())
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, mut mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
        let m = &self.abort_signal;
        if !m.is_undefined() {
            mark_fn(m.as_jsvalue());
        }
    }
}
