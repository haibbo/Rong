#![doc = "Raw FFI bindings to V8"]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/v8_bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_v8_binding() {
        unsafe {}
    }
}
