#![doc = "Raw FFI bindings to QuickJS-NG"]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/quickjs.bindings.rs"));

#[cfg(test)]
mod tests {

    use super::*;
    use std::ffi::CString;

    #[test]
    fn run_script() {
        let js = "63";
        let mut number: i64 = 0;

        unsafe {
            let rt = JS_NewRuntime();
            let ctx = JS_NewContext(rt);
            let c_string = CString::new(js).unwrap();
            let jsvalue = JS_Eval(
                ctx,
                c_string.as_ptr(),
                js.len() as _,
                c"eval".as_ptr(),
                0 as _,
            );
            JS_ToInt64(ctx, &mut number, jsvalue);
            JS_FreeValue(ctx, jsvalue);
            JS_FreeContext(ctx);
            JS_FreeRuntime(rt);
        }
        assert!(number == 63);
    }
}
