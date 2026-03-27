use crate::{
    JSContext, JSFunc, JSObject, JSObjectOps, JSResult, JSSymbol, JSValue, JSValueConversion,
    JSValueImpl, RongJSError,
};

// PropertyKey represents a key in a JavaScript object property
// It can be a number (i32, u32, i64, u64) or a string reference
#[derive(Clone)]
pub enum PropertyKey<'a, V: JSValueImpl> {
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Uint64(u64),
    Str(&'a str),
    Symbol(JSSymbol<V>),
}

impl<V: JSValueImpl> From<i32> for PropertyKey<'_, V> {
    fn from(value: i32) -> Self {
        PropertyKey::Int32(value)
    }
}

impl<V: JSValueImpl> From<u32> for PropertyKey<'_, V> {
    fn from(value: u32) -> Self {
        PropertyKey::Uint32(value)
    }
}

impl<V: JSValueImpl> From<i64> for PropertyKey<'_, V> {
    fn from(value: i64) -> Self {
        PropertyKey::Int64(value)
    }
}

impl<V: JSValueImpl> From<u64> for PropertyKey<'_, V> {
    fn from(value: u64) -> Self {
        PropertyKey::Uint64(value)
    }
}

impl<V: JSValueImpl> From<JSSymbol<V>> for PropertyKey<'_, V> {
    fn from(value: JSSymbol<V>) -> Self {
        PropertyKey::Symbol(value)
    }
}

// The key implementation for handling string property keys
// 'b: 'a means the input string's lifetime ('b) must outlive or equal the PropertyKey's lifetime ('a)
// This ensures the string reference stored in PropertyKey remains valid throughout PropertyKey's lifetime
impl<'a, 'b: 'a, V: JSValueImpl> From<&'b str> for PropertyKey<'a, V> {
    fn from(value: &'b str) -> Self {
        PropertyKey::Str(value)
    }
}

// Convert PropertyKey into the actual JavaScript value type
// No lifetime bound needed here as we're consuming self
impl<V: JSValueImpl> PropertyKey<'_, V> {
    pub(crate) fn into_value(self, context: &JSContext<V::Context>) -> V
    where
        V: JSValueConversion,
    {
        let ctx = context.as_ref();
        match self {
            Self::Int32(i) => (ctx, i).into(),
            Self::Uint32(i) => (ctx, i).into(),
            Self::Int64(i) => (ctx, i).into(),
            Self::Uint64(i) => (ctx, i).into(),
            Self::Str(s) => (ctx, s).into(),
            Self::Symbol(s) => JSSymbol::into_value(s),
        }
    }
}

impl<V: JSObjectOps> std::fmt::Display for PropertyKey<'_, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PropertyKey::Int32(i) => write!(f, "{}", i),
            PropertyKey::Uint32(i) => write!(f, "{}", i),
            PropertyKey::Int64(i) => write!(f, "{}", i),
            PropertyKey::Uint64(i) => write!(f, "{}", i),
            PropertyKey::Str(s) => write!(f, "{}", s),
            PropertyKey::Symbol(s) => write!(
                f,
                "Symbol({})",
                s.descripiton().unwrap_or_else(|_| "".to_string())
            ),
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct PropertyAttributes(u32);

impl PropertyAttributes {
    const WRITABLE: u32 = 1;
    const ENUMERABLE: u32 = 1 << 1;
    const CONFIGURABLE: u32 = 1 << 2;
    const HAS_VALUE: u32 = 1 << 3;
    const HAS_GET: u32 = 1 << 4;
    const HAS_SET: u32 = 1 << 5;
    const HAS_WRITABLE: u32 = 1 << 6;
    const HAS_ENUMERABLE: u32 = 1 << 7;
    const HAS_CONFIGURABLE: u32 = 1 << 8;

    pub fn is_writable(&self) -> bool {
        self.0 & Self::WRITABLE != 0
    }

    #[doc(hidden)]
    pub fn has_writable(&self) -> bool {
        self.0 & Self::HAS_WRITABLE != 0
    }

    pub fn is_enumerable(&self) -> bool {
        self.0 & Self::ENUMERABLE != 0
    }

    #[doc(hidden)]
    pub fn has_enumerable(&self) -> bool {
        self.0 & Self::HAS_ENUMERABLE != 0
    }

    pub fn is_configurable(&self) -> bool {
        self.0 & Self::CONFIGURABLE != 0
    }

    #[doc(hidden)]
    pub fn has_configurable(&self) -> bool {
        self.0 & Self::HAS_CONFIGURABLE != 0
    }

    pub fn has_value(&self) -> bool {
        self.0 & Self::HAS_VALUE != 0
    }

    pub fn has_get(&self) -> bool {
        self.0 & Self::HAS_GET != 0
    }

    pub fn has_set(&self) -> bool {
        self.0 & Self::HAS_SET != 0
    }
}

pub struct PropertyDescriptor<V: JSValueImpl> {
    value: Option<V>,
    getter: Option<JSFunc<V>>,
    setter: Option<JSFunc<V>>,
    attributes: PropertyAttributes,
}

impl<V> PropertyDescriptor<V>
where
    V: JSObjectOps,
{
    fn thrown_error(ctx: &JSContext<V::Context>, thrown: V) -> RongJSError {
        RongJSError::from_thrown_value(JSValue::from_raw(ctx, thrown))
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            value: None,
            getter: None,
            setter: None,
            attributes: PropertyAttributes::default(),
        }
    }

    #[must_use]
    pub fn from_value(value: JSValue<V>) -> Self {
        Self::new().value(value)
    }

    #[must_use]
    pub fn from_rust<T>(ctx: &JSContext<V::Context>, value: T) -> Self
    where
        T: crate::IntoJSValue<V>,
    {
        Self::from_value(JSValue::from_rust(ctx, value))
    }

    #[must_use]
    pub fn from_getter(getter: JSFunc<V>) -> Self {
        Self::new().getter(getter)
    }

    #[must_use]
    pub fn from_setter(setter: JSFunc<V>) -> Self {
        Self::new().setter(setter)
    }

    #[must_use]
    pub fn from_accessor(getter: JSFunc<V>, setter: JSFunc<V>) -> Self {
        Self::from_getter(getter).setter(setter)
    }

    #[must_use]
    pub fn value(mut self, value: JSValue<V>) -> Self {
        self.value = Some(value.into_value());
        self
    }

    #[must_use]
    pub fn getter(mut self, getter: JSFunc<V>) -> Self {
        self.getter = Some(getter);
        self
    }

    #[must_use]
    pub fn setter(mut self, setter: JSFunc<V>) -> Self {
        self.setter = Some(setter);
        self
    }

    #[must_use]
    pub fn writable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_WRITABLE;
        self.attributes.0 |= PropertyAttributes::WRITABLE;
        self
    }

    #[must_use]
    pub fn readonly(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_WRITABLE;
        self.attributes.0 &= !PropertyAttributes::WRITABLE;
        self
    }

    #[must_use]
    pub fn enumerable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_ENUMERABLE;
        self.attributes.0 |= PropertyAttributes::ENUMERABLE;
        self
    }

    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_ENUMERABLE;
        self.attributes.0 &= !PropertyAttributes::ENUMERABLE;
        self
    }

    #[must_use]
    pub fn configurable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_CONFIGURABLE;
        self.attributes.0 |= PropertyAttributes::CONFIGURABLE;
        self
    }

    #[must_use]
    pub fn non_configurable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::HAS_CONFIGURABLE;
        self.attributes.0 &= !PropertyAttributes::CONFIGURABLE;
        self
    }

    pub fn define_on<'a, K>(mut self, obj: &JSObject<V>, k: K) -> JSResult<()>
    where
        K: Into<PropertyKey<'a, V>>,
        V: JSObjectOps,
    {
        let ctx = &obj.context();
        let undefined = V::create_undefined(ctx.as_ref()); // UNDEFINED

        let value = self
            .value
            .inspect(|_| self.attributes.0 |= PropertyAttributes::HAS_VALUE)
            .unwrap_or(undefined.clone());

        let getter = self
            .getter
            .map(|g| {
                self.attributes.0 |= PropertyAttributes::HAS_GET;
                g.into_value()
            })
            .unwrap_or(undefined.clone());

        let setter = self
            .setter
            .map(|s| {
                self.attributes.0 |= PropertyAttributes::HAS_SET;
                s.into_value()
            })
            .unwrap_or(undefined.clone());

        let key = k.into().into_value(ctx);

        obj.as_value()
            .define_property(key, value, getter, setter, self.attributes)
            .map_err(|thrown| Self::thrown_error(ctx, thrown))
    }
}
