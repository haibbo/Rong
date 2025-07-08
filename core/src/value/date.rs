use crate::*;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct JSDate<V: JSValueImpl> {
    inner: JSValue<V>,
}

impl<V: JSValueImpl> JSDate<V> {
    /// Create a new JSDate from epoch milliseconds
    pub fn new(ctx: &JSContext<V::Context>, epoch_ms: f64) -> Self {
        let value = V::create_date(ctx.as_ref(), epoch_ms);
        Self {
            inner: JSValue::from_raw(ctx, value),
        }
    }

    /// Create a JSDate for the current time
    pub fn now(ctx: &JSContext<V::Context>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f64;
        Self::new(ctx, now)
    }

    /// Create a JSDate from SystemTime
    pub fn from_system_time(ctx: &JSContext<V::Context>, time: SystemTime) -> Self {
        let epoch_ms = time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f64;
        Self::new(ctx, epoch_ms)
    }

    /// Get the epoch milliseconds by calling JavaScript getTime() method
    pub fn get_time(&self) -> JSResult<f64>
    where
        V: JSValueImpl + JSTypeOf + JSValueConversion + JSObjectOps,
    {
        // Convert to JSObject to access methods
        let date_obj = self
            .inner
            .clone()
            .into_object()
            .ok_or_else(|| RongJSError::TypeError("Date is not an object".into()))?;

        // Get the getTime method
        let get_time = date_obj.get::<_, JSFunc<V>>("getTime")?;

        // Call getTime() method with the date object as 'this'
        get_time.call::<_, f64>(Some(date_obj), ())
    }

    /// Convert to SystemTime
    pub fn to_system_time(&self) -> JSResult<SystemTime>
    where
        V: JSValueImpl + JSTypeOf + JSValueConversion + JSObjectOps,
    {
        let epoch_ms = self.get_time()?;
        let duration = std::time::Duration::from_millis(epoch_ms as u64);
        Ok(UNIX_EPOCH + duration)
    }

    /// Get the underlying JSValue
    pub fn as_value(&self) -> &JSValue<V> {
        &self.inner
    }

    /// Convert into the underlying JSValue
    pub fn into_value(self) -> JSValue<V> {
        self.inner
    }
}

impl<V: JSValueImpl> From<JSDate<V>> for JSValue<V> {
    fn from(date: JSDate<V>) -> Self {
        date.inner
    }
}

impl<V: JSValueImpl> FromJSValue<V> for JSDate<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        let js_value = JSValue::from_raw(ctx, value);
        if !js_value.is_date() {
            return Err(RongJSError::TypeError("Value is not a Date".into()));
        }
        Ok(JSDate { inner: js_value })
    }
}

impl<V: JSValueImpl> IntoJSValue<V> for JSDate<V> {
    fn into_js_value(self, _ctx: &JSContext<V::Context>) -> V {
        self.inner.into_value()
    }
}

// Support for SystemTime conversion
impl<V: JSValueImpl> FromJSValue<V> for SystemTime
where
    V: JSTypeOf + JSValueConversion + JSObjectOps,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        let js_date = JSDate::from_js_value(ctx, value)?;
        js_date.to_system_time()
    }
}

impl<V: JSValueImpl> IntoJSValue<V> for SystemTime {
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        let epoch_ms = self
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as f64;
        V::create_date(ctx.as_ref(), epoch_ms)
    }
}
