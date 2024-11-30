#![doc = "Raw FFI bindings to JavaScriptCore"]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod test {

    use super::*;
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_jscore_raw_binding() {
        unsafe {
            let global_context = JSGlobalContextCreate(ptr::null_mut());

            let js_code = CString::new("Math.sqrt(16)").expect("CString::new failed");

            let js_string = JSStringCreateWithUTF8CString(js_code.as_ptr());

            let mut exception: JSValueRef = ptr::null_mut();

            let result = JSEvaluateScript(
                global_context,
                js_string,
                ptr::null_mut(), // thisObject, use null for global
                ptr::null_mut(), // sourceURL
                1,               // startingLineNumber
                &mut exception,
            );

            if !exception.is_null() {
                let exception_string =
                    JSValueToStringCopy(global_context, exception, ptr::null_mut());
                let exception_cstring = JSStringGetCharactersPtr(exception_string);
                println!("JavaScript exception occurred: {:?}", exception_cstring);
                JSStringRelease(exception_string);
            } else {
                let result_number = JSValueToInt32(global_context, result, ptr::null_mut());
                assert_eq!(result_number, 4);
            }

            JSStringRelease(js_string);
            JSGlobalContextRelease(global_context);
        }
    }
}
