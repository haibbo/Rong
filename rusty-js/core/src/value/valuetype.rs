use crate::{JSValue, JSValueKind};

#[derive(Clone, Debug)]
pub enum ValueType {
    Undefined,
    Null,
    Error,
    Exception,
    Boolean,
    Number,
    BigInt,
    String,
    Object,
    Array,
    Function,
    Promise,
    Symbol,
    Unknown,
}

pub trait JSTypeOf {
    fn is_exception(&self) -> bool;
    fn is_error(&self) -> bool;
    fn is_array(&self) -> bool;
    fn is_promise(&self) -> bool;
    fn is_undefined(&self) -> bool;
    fn is_null(&self) -> bool;
    fn is_boolean(&self) -> bool;
    fn is_number(&self) -> bool;
    fn is_bigint(&self) -> bool;
    fn is_string(&self) -> bool;
    fn is_symbol(&self) -> bool;
    fn is_function(&self) -> bool;
    fn is_object(&self) -> bool;

    fn type_of(&self) -> ValueType {
        if self.is_exception() {
            ValueType::Exception
        } else if self.is_error() {
            ValueType::Error
        } else if self.is_promise() {
            ValueType::Promise
        } else if self.is_array() {
            ValueType::Array
        } else if self.is_function() {
            ValueType::Function
        } else if self.is_object() {
            ValueType::Object
        } else if self.is_undefined() {
            ValueType::Undefined
        } else if self.is_null() {
            ValueType::Null
        } else if self.is_boolean() {
            ValueType::Boolean
        } else if self.is_number() {
            ValueType::Number
        } else if self.is_bigint() {
            ValueType::BigInt
        } else if self.is_string() {
            ValueType::String
        } else if self.is_symbol() {
            ValueType::Symbol
        } else {
            ValueType::Unknown
        }
    }
}

impl<'ctx, V> JSValue<'ctx, V>
where
    V: JSValueKind,
{
    pub fn is_exception(&self) -> bool {
        self.raw.is_exception()
    }

    pub fn is_error(&self) -> bool {
        self.raw.is_error()
    }

    pub fn is_array(&self) -> bool {
        self.raw.is_array()
    }

    pub fn is_promise(&self) -> bool {
        self.raw.is_promise()
    }

    pub fn is_undefined(&self) -> bool {
        self.raw.is_undefined()
    }

    pub fn is_null(&self) -> bool {
        self.raw.is_null()
    }

    pub fn is_boolean(&self) -> bool {
        self.raw.is_boolean()
    }

    pub fn is_number(&self) -> bool {
        self.raw.is_number()
    }

    pub fn is_bigint(&self) -> bool {
        self.raw.is_bigint()
    }

    pub fn is_string(&self) -> bool {
        self.raw.is_string()
    }

    pub fn is_symbol(&self) -> bool {
        self.raw.is_symbol()
    }

    pub fn is_function(&self) -> bool {
        self.raw.is_function()
    }

    pub fn is_object(&self) -> bool {
        self.raw.is_object()
    }

    pub fn type_of(&self) -> ValueType {
        self.raw.type_of()
    }
}
