use crate::AbortSignal;
use rusty_js::{function::*, *};

#[js_export]
pub struct AbortController {
    abort_signal: JSObject, // AbortSignal
}

#[js_methods]
impl AbortController {
    #[js_method(constructor)]
    fn new(ctx: JSContext) -> JSResult<Self> {
        Ok(Self {
            abort_signal: Class::get::<AbortSignal>(&ctx)?.instance(AbortSignal::new()),
        })
    }

    #[js_method(getter)]
    pub fn signal(&self) -> JSObject {
        self.abort_signal.clone()
    }

    #[js_method]
    pub fn abort(&self, ctx: JSContext, reason: Optional<JSValue>) -> JSResult<()> {
        let mut abort = self.abort_signal.borrow_mut::<AbortSignal>()?;
        if abort.aborted() {
            //only once
            return Ok(());
        }
        abort.set_reason(reason);
        drop(abort);
        AbortSignal::send_aborted(&ctx, This(self.abort_signal.clone()))?;
        Ok(())
    }
}
