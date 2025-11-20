use rong_test::*;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
struct TestContextService {
    flag: Arc<AtomicBool>,
}

impl JSContextService for TestContextService {
    fn on_shutdown(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }
}

#[test]
fn context_service_shutdown_is_called_on_drop() {
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    {
        let flag = shutdown_flag.clone();
        run(|ctx| {
            let service = TestContextService { flag };
            ctx.set_service::<TestContextService>(service);
            Ok(())
        });
        // JSContext is dropped at the end of run(), which should trigger on_shutdown.
    }
    assert!(
        shutdown_flag.load(Ordering::SeqCst),
        "JSContextService::on_shutdown was not called on context drop"
    );
}
