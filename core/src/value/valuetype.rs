use crate::{JSValue, JSValueImpl};
use std::fmt;

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
    ArrayBuffer,
    Function,
    Constructor,
    Promise,
    Symbol,
    Date,
    Unknown,
}

impl fmt::Display for JSValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            JSValueType::Undefined => "undefined",
            JSValueType::Null => "null",
            JSValueType::Error => "error",
            JSValueType::Exception => "exception",
            JSValueType::Boolean => "boolean",
            JSValueType::Number => "number",
            JSValueType::BigInt => "bigint",
            JSValueType::String => "string",
            JSValueType::Object => "object",
            JSValueType::Array => "array",
            JSValueType::ArrayBuffer => "arrayBuffer",
            JSValueType::Function => "function",
            JSValueType::Constructor => "constructor",
            JSValueType::Promise => "promise",
            JSValueType::Symbol => "symbol",
            JSValueType::Date => "Date",
            JSValueType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

pub trait JSTypeOf: JSValueImpl {
    fn is_exception(&self) -> bool;
    fn is_error(&self) -> bool;
    fn is_array(&self) -> bool;
    fn is_array_buffer(&self) -> bool;
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
    fn is_date(&self) -> bool;

    fn type_of(&self) -> JSValueType {
        if self.is_exception() {
            JSValueType::Exception
        } else if self.is_error() {
            JSValueType::Error
        } else if self.is_promise() {
            JSValueType::Promise
        } else if self.is_array() {
            JSValueType::Array
        } else if self.is_array_buffer() {
            JSValueType::ArrayBuffer
        } else if self.is_function() {
            JSValueType::Function
        } else if self.is_constructor() {
            JSValueType::Constructor
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
        } else if self.is_date() {
            JSValueType::Date
        } else if self.is_symbol() {
            JSValueType::Symbol
        } else if self.is_object() {
            // check is_object at last stage, since such as function etc is also object
            JSValueType::Object
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
    ($($take_method: ident => $is_method: ident),*) => {
        impl<V> JSValue<V>
        where
            V: JSTypeOf,
        {
            $(
                pub fn $take_method(self) -> Option<Self> {
                    if self.inner.$is_method() {
                        Some(self)
                    } else {
                        None
                    }
                }

                pub fn $is_method(&self) -> bool {
                    self.inner.$is_method()
                }
            )*
        }
    }
}

generate_is_type!(
    take_is_object => is_object,
    take_is_array => is_array,
    take_is_array_buffer => is_array_buffer,
    take_is_function => is_function,
    take_is_constructor => is_constructor,
    take_is_promise => is_promise,
    take_is_error => is_error,
    take_is_exception => is_exception,
    take_is_undefined => is_undefined,
    take_is_null => is_null,
    take_is_boolean => is_boolean,
    take_is_number => is_number,
    take_is_bigint => is_bigint,
    take_is_string => is_string,
    take_is_symbol => is_symbol,
    take_is_date => is_date
);
