use crate::{JSValue, JSValueImpl};

#[derive(Clone, Debug)]
pub enum JSValueType {
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

    fn type_of(&self) -> JSValueType {
        if self.is_exception() {
            JSValueType::Exception
        } else if self.is_error() {
            JSValueType::Error
        } else if self.is_promise() {
            JSValueType::Promise
        } else if self.is_array() {
            JSValueType::Array
        } else if self.is_function() {
            JSValueType::Function
        } else if self.is_constructor() {
            JSValueType::Constructor
        } else if self.is_object() {
            JSValueType::Object
        } else if self.is_undefined() {
            JSValueType::Undefined
        } else if self.is_null() {
            JSValueType::Null
        } else if self.is_boolean() {
            JSValueType::Boolean
        } else if self.is_number() {
            JSValueType::Number
        } else if self.is_bigint() {
            JSValueType::BigInt
        } else if self.is_string() {
            JSValueType::String
        } else if self.is_symbol() {
            JSValueType::Symbol
        } else {
            JSValueType::Unknown
        }
    }
}

impl<V> JSValue<V>
where
    V: JSTypeOf,
{
    pub fn type_of(&self) -> JSValueType {
        self.inner.type_of()
    }
}

macro_rules! generate_is_type {
    ($($method: ident),*) => {
        impl<V> JSValue<V>
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
    is_error,
    is_exception,
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
