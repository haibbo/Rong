use crate::JSRtInner;

pub struct JSRuntime {
    pub(crate) inner: JSRtInner,
}

impl JSRuntime {
    pub fn new() -> Result<Self, String> {
        JSRtInner::new().map(|inner| Self { inner })
    }
}
