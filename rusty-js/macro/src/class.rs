use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Result};

/// Options for the class macro (currently unused but kept for future expansion)
#[derive(Default)]
pub struct ClassOpts {
    // Add options here as needed
}

/// Main implementation of the class macro
pub fn class_impl(input: &ItemStruct, _opts: &ClassOpts) -> Result<TokenStream> {
    // Get the struct name
    let name = &input.ident;

    let expanded = quote! {

        #[derive(Clone,Copy)]
        #input

        impl rusty_js::IntoJSValue<rusty_js::JSEngineValue> for #name {

            fn into_js_value(self, context: &rusty_js::JSContext) -> rusty_js::JSEngineValue {
                rusty_js::Class::get::<Self>(context)
                    .map(|class| class.instance(self))
                    .unwrap_or_else(|| rusty_js::JSEngineValue::from((context.as_ref(), ())))
            }
        }

        impl rusty_js::FromJSValue<rusty_js::JSEngineValue> for #name {
            fn from_js_value(ctx: &rusty_js::JSContext, value: rusty_js::JSEngineValue) -> rusty_js::JSResult<Self> {
                let obj = rusty_js::JSObject::from_js_value(ctx, value)?;
                let instance = obj.borrow::<Self>()?;
                Ok(*instance)
            }
        }

        impl rusty_js::function::JSParameterType for #name {}
    };

    Ok(expanded)
}
