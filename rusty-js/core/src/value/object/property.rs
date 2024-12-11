use crate::{JSContext, JSFunc, JSObject, JSObjectOps, JSValueConversion, JSValueImpl};

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

impl<'a> From<&'a str> for PropertyKey<'a> {
    fn from(value: &'a str) -> Self {
        PropertyKey::Str(value)
    }
}

impl<'ctx> PropertyKey<'ctx> {
    pub fn into_key<V>(self, ctx: &'ctx JSContext<V::Context>) -> V
    where
        V: JSValueConversion,
    {
        match self {
            Self::Int32(i) => (&ctx.inner, i).into(),
            Self::Uint32(i) => (&ctx.inner, i).into(),
            Self::Int64(i) => (&ctx.inner, i).into(),
            Self::Uint64(i) => (&ctx.inner, i).into(),
            Self::Str(s) => (&ctx.inner, s).into(),
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

pub struct PropertyDescriptor<'ctx, V: JSValueImpl> {
    name: String,
    value: Option<V>,
    getter: Option<JSFunc<'ctx, V>>,
    setter: Option<JSFunc<'ctx, V>>,
    attributes: PropertyAttributes,
}

impl<'ctx, V> PropertyDescriptor<'ctx, V>
where
    V: JSObjectOps<'ctx>,
{
    #[must_use]
    pub fn builder(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: None,
            getter: None,
            setter: None,
            attributes: PropertyAttributes::default(),
        }
    }

    #[must_use]
    pub fn getter(mut self, getter: JSFunc<'ctx, V>) -> Self {
        self.getter = Some(getter);
        self
    }

    #[must_use]
    pub fn setter(mut self, setter: JSFunc<'ctx, V>) -> Self {
        self.setter = Some(setter);
        self
    }

    #[must_use]
    fn writable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::WRITABLE;
        self
    }

    #[must_use]
    fn enumerable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::ENUMERABLE;
        self
    }

    #[must_use]
    fn configurable(mut self) -> Self {
        self.attributes.0 |= PropertyAttributes::CONFIGURABLE;
        self
    }

    // apply PropertyDescriptor to JS Object with key
    pub fn apply_to<K>(mut self, obj: &JSObject<'ctx, V>, k: K)
    where
        K: Into<PropertyKey<'ctx>>,
        V: JSObjectOps<'ctx>,
    {
        let value = self
            .value
            .inspect(|_| self.attributes.0 |= PropertyAttributes::HAS_VALUE)
            .unwrap_or(V::from((obj.as_ctx().as_inner(), ()))); //UNDEFIEND

        let getter = self
            .getter
            .map(|g| {
                self.attributes.0 |= PropertyAttributes::HAS_GET;
                g.into_inner()
            })
            .unwrap_or(V::from((obj.as_ctx().as_inner(), ()))); //UNDEFIEND

        let setter = self
            .setter
            .map(|s| {
                self.attributes.0 |= PropertyAttributes::HAS_SET;
                s.into_inner()
            })
            .unwrap_or(V::from((obj.as_ctx().as_inner(), ()))); // UNDEFINED

        let key = k.into().into_key(obj.as_ctx());

        obj.as_inner()
            .define_property(key, value, getter, setter, self.attributes);
    }
}
