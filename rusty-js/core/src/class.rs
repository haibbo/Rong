use crate::{JSValueImpl, RustCallable, RustFunc};

/// JSClass trait for rust type
pub trait JSClass<V: JSValueImpl>: Sized {
    // the name of class constructor
    const NAME: &'static str;

    fn data_constructor() -> RustFunc<V>;
}

pub trait JSClassExt<V: JSValueImpl>: JSClass<V> {
    fn constructor(context: &V::Context, args: &[V]) -> V {
        Self::data_constructor().call(context, args).unwrap()
    }
}

// Blanket implementation
impl<T, V: JSValueImpl> JSClassExt<V> for T where T: JSClass<V> {}
