use crate::{JSContext, JSFunc, JSObject, JSObjectOps, JSValueConversion, JSValueImpl};

// PropertyKey represents a key in a JavaScript object property
// It can be a number (i32, u32, i64, u64) or a string reference
pub enum PropertyKey<'a> {
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Uint64(u64),
    Str(&'a str),
    // Symbol(Symbol),
}

impl From<i32> for PropertyKey<'_> {
    fn from(value: i32) -> Self {
        PropertyKey::Int32(value)
    }
}

impl From<u32> for PropertyKey<'_> {
    fn from(value: u32) -> Self {
        PropertyKey::Uint32(value)
    }
}

impl From<i64> for PropertyKey<'_> {
    fn from(value: i64) -> Self {
        PropertyKey::Int64(value)
    }
}

impl From<u64> for PropertyKey<'_> {
    fn from(value: u64) -> Self {
        PropertyKey::Uint64(value)
    }
}

// The key implementation for handling string property keys
// 'b: 'a means the input string's lifetime ('b) must outlive or equal the PropertyKey's lifetime ('a)
// This ensures the string reference stored in PropertyKey remains valid throughout PropertyKey's lifetime
impl<'a, 'b: 'a> From<&'b str> for PropertyKey<'a> {
    fn from(value: &'b str) -> Self {
        PropertyKey::Str(value)
    }
}

// Convert PropertyKey into the actual JavaScript value type
// No lifetime bound needed here as we're consuming self
impl PropertyKey<'_> {
    pub(crate) fn into_key<V>(self, context: &JSContext<V::Context>) -> V
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

    pub fn is_writable(&self) -> bool {
        self.0 & Self::WRITABLE != 0
    }

    pub fn is_enumerable(&self) -> bool {
        self.0 & Self::ENUMERABLE != 0
    }

    pub fn is_configurable(&self) -> bool {
        self.0 & Self::CONFIGURABLE != 0
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
    #[must_use]
    pub(crate) fn builder() -> Self {
        Self {
            value: None,
            getter: None,
            setter: None,
            attributes: PropertyAttributes::default(),
        }
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
        self.attributes.0 |= PropertyAttributes::WRITABLE;
        self
    }

    #[must_use]
    pub fn enumerable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::ENUMERABLE;
        self
    }

    #[must_use]
    pub fn configurable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::CONFIGURABLE;
        self
    }

    #[must_use]
    pub fn with_default_method_attr(mut self) -> Self {
        self.attributes.0 = PropertyAttributes::WRITABLE | PropertyAttributes::CONFIGURABLE;
        self
    }

    #[must_use]
    pub fn wth_default_property_attr(mut self) -> Self {
        self.attributes.0 = PropertyAttributes::WRITABLE
            | PropertyAttributes::CONFIGURABLE
            | PropertyAttributes::ENUMERABLE;
        self
    }

    // apply PropertyDescriptor to JS Object with key
    pub(crate) fn apply_to<K>(mut self, obj: &JSObject<V>, k: K)
    where
        K: for<'a> Into<PropertyKey<'a>>,
        V: JSObjectOps,
    {
        let ctx = &obj.get_ctx();
        let undefined = V::from((ctx.as_ref(), ())); // UNDEFINED

        let value = self
            .value
            .inspect(|_| self.attributes.0 |= PropertyAttributes::HAS_VALUE)
            .unwrap_or(undefined.clone());

        let getter = self
            .getter
            .map(|g| {
                self.attributes.0 |= PropertyAttributes::HAS_GET;
                g.into_inner()
            })
            .unwrap_or(undefined.clone());

        let setter = self
            .setter
            .map(|s| {
                self.attributes.0 |= PropertyAttributes::HAS_SET;
                s.into_inner()
            })
            .unwrap_or(undefined.clone());

        let key = k.into().into_key(ctx);

        obj.as_value()
            .define_property(key, value, getter, setter, self.attributes);
    }
}
