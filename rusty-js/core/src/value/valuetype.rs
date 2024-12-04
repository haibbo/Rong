use crate::{JSValue, JSValueImpl};

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
    Constructor,
    Promise,
    Symbol,
    Unknown,
}

pub trait JSTypeOf: JSValueImpl {
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
    fn is_constructor(&self) -> bool;

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
        } else if self.is_constructor() {
            ValueType::Constructor
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
    V: JSTypeOf,
{
    pub fn type_of(&self) -> ValueType {
        self.inner.type_of()
    }
}

macro_rules! generate_is_type {
    ($($method: ident),*) => {
        impl<'ctx, V> JSValue<'ctx, V>
        where
            V: JSTypeOf,
        {
            $(
                pub fn $method(&self) -> Option<&Self> {
                    if self.inner.$method() {
                        Some(self)
                    }else{
                        None
                    }
                }
            )*
        }
    }
}

generate_is_type!(
    is_exception,
    is_error,
    is_array,
    is_promise,
    is_function,
    is_constructor,
    is_object,
    is_undefined,
    is_null,
    is_boolean,
    is_number,
    is_bigint,
    is_string,
    is_symbol
);
