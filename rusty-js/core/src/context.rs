use crate::IntoHost;
use anyhow;

pub trait JSCtxExt<'ctx>: Sized {
    type Value;
    fn eval<S, T>(&'ctx self, source: S) -> anyhow::Result<T>
    where
        S: AsRef<str>,
        Self::Value: IntoHost<T>;
}
