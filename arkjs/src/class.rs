use crate::{ArkJSContext, ArkJSValue, arkjs};
use rong_core::{JSClass, JSClassExt, JSContextImpl, JSTypeOf, JSValueImpl};
use std::collections::HashMap;
use std::ffi::CString;
use std::ptr;
use std::sync::{LazyLock, RwLock};

/// Global storage mapping constructor objects to their corresponding class references
static CLASS: LazyLock<RwLock<HashMap<usize, usize>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

unsafe extern "C" fn generic_constructor<JC>(
    env: arkjs::JSVM_Env,
    info: arkjs::JSVM_CallbackInfo,
) -> arkjs::JSVM_Value
where
    JC: JSClass<ArkJSValue>,
{
    unsafe {
        let mut argc: usize = 0;
        let argv: *mut arkjs::JSVM_Value = ptr::null_mut();
        let mut this_arg: arkjs::JSVM_Value = ptr::null_mut();
        let mut data: *mut std::ffi::c_void = ptr::null_mut();

        let status = arkjs::OH_JSVM_GetCbInfo(env, info, &mut argc, argv, &mut this_arg, &mut data);

        if status != arkjs::JSVM_Status_JSVM_OK {
            let mut undefined: arkjs::JSVM_Value = ptr::null_mut();
            arkjs::OH_JSVM_GetUndefined(env, &mut undefined);
            return undefined;
        }

        let ctx = ArkJSContext::from_borrowed_raw(env);
        let this = ArkJSValue::from_borrowed_raw(env, this_arg);

        let args: Vec<ArkJSValue> = if argc > 0 && !argv.is_null() {
            (0..argc)
                .map(|i| ArkJSValue::from_borrowed_raw(env, *argv.add(i)))
                .collect()
        } else {
            vec![]
        };

        let value = <JC as JSClassExt<ArkJSValue>>::constructor(&ctx, this, args);
        if value.is_exception() {
            let mut undefined: arkjs::JSVM_Value = ptr::null_mut();
            arkjs::OH_JSVM_GetUndefined(env, &mut undefined);
            return undefined;
        }

        value.into_raw_value()
    }
}

// Typed finalizer for specific class JC
// Use finalizeHint as JSVM_Value to reconstruct ArkJSValue and call JSClassExt::free properly
unsafe extern "C" fn finalizer<JC>(
    env: arkjs::JSVM_Env,
    _finalize_data: *mut std::ffi::c_void,
    finalize_hint: *mut std::ffi::c_void,
) where
    JC: JSClass<ArkJSValue>,
{
    if !finalize_hint.is_null() {
        // finalize_hint is the JS instance value (JSVM_Value) that we passed in make_instance
        // Since JSVM_Value is typedef struct JSVM_Value__*, we can cast it directly
        let js_value = finalize_hint as arkjs::JSVM_Value;

        // Reconstruct ArkJSValue from the JS value
        let value = ArkJSValue::from_borrowed_raw(env, js_value);

        <JC as JSClassExt<ArkJSValue>>::free(value);
    }
}

// Class registration for ArkJS using OH_JSVM_DefineClass
pub(crate) fn register_class_internal<JC>(ctx: &ArkJSContext, class_name: &str) -> ArkJSValue
where
    JC: JSClass<ArkJSValue>,
{
    unsafe {
        let class_name_cstr = CString::new(class_name).unwrap();
        let mut constructor: arkjs::JSVM_Value = ptr::null_mut();

        // Create constructor callback
        let constructor_callback = arkjs::JSVM_CallbackStruct {
            callback: Some(generic_constructor::<JC>),
            data: ptr::null_mut(),
        };

        // Create properties array for methods and static properties
        let properties: Vec<arkjs::JSVM_PropertyDescriptor> = vec![];

        // Define the class using OH_JSVM_DefineClass
        let status = arkjs::OH_JSVM_DefineClass(
            ctx.to_raw(),
            class_name_cstr.as_ptr(),
            class_name.len(),
            &constructor_callback as *const _ as *mut _,
            properties.len(),
            if properties.is_empty() {
                ptr::null()
            } else {
                properties.as_ptr()
            },
            &mut constructor,
        );

        if status == arkjs::JSVM_Status_JSVM_OK {
            // Set constructor name property
            let name_key = CString::new("name").unwrap();
            let mut name_value: arkjs::JSVM_Value = ptr::null_mut();
            arkjs::OH_JSVM_CreateStringUtf8(
                ctx.to_raw(),
                class_name_cstr.as_ptr(),
                class_name.len(),
                &mut name_value,
            );

            arkjs::OH_JSVM_SetNamedProperty(
                ctx.to_raw(),
                constructor,
                name_key.as_ptr(),
                name_value,
            );

            // Store class mapping for future reference
            // Map constructor pointer to finalizer function pointer
            let finalizer_ptr = finalizer::<JC> as *const std::ffi::c_void as usize;
            CLASS
                .write()
                .unwrap()
                .insert(constructor as usize, finalizer_ptr);

            ArkJSValue::from_owned_raw(ctx.to_raw(), constructor).with_object()
        } else {
            ArkJSValue::create_undefined(ctx)
        }
    }
}

/// Retrieves the finalizer function pointer associated with a given constructor
pub(crate) fn get_finalizer_by_constructor(constructor: ArkJSValue) -> arkjs::JSVM_Finalize {
    let constructor_ptr = constructor.as_raw_value() as *const _ as usize;
    if let Ok(map) = CLASS.read() {
        // In ArkJS, we store the finalizer function pointer instead of class ref
        if let Some(&finalizer_ptr) = map.get(&constructor_ptr) {
            unsafe {
                return Some(std::mem::transmute::<
                    usize,
                    unsafe extern "C" fn(
                        arkjs::JSVM_Env,
                        *mut std::ffi::c_void,
                        *mut std::ffi::c_void,
                    ),
                >(finalizer_ptr));
            }
        }
    }
    None
}
