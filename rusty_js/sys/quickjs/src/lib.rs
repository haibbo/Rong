#![allow(non_upper_case_globals)]

include!(concat!(env!("OUT_DIR"), "/quickjs.bindings.rs"));

#[cfg(test)]
mod tests {

    use super::*;
    use std::ffi::CString;
    use std::os::raw::c_char;

    #[test]
    fn run_script() {
        let js = "63";
        let mut number: i64 = 0;

        unsafe {
            let rt = JS_NewRuntime();
            let ctx = JS_NewContext(rt);
            let c_string = CString::new(js).unwrap();
            let jsvalue = QJS_RunScript(ctx, c_string.as_ptr() as *mut c_char, js.len() as i32);
            JS_ToInt64(ctx, &mut number, jsvalue);
            JS_FreeValue(ctx, jsvalue);
            JS_FreeContext(ctx);
            JS_FreeRuntime(rt);
        }
        assert!(number == 63);
    }
}
