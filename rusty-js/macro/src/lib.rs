use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for implementing JavaScript value conversion traits
/// This will implement:
/// - IntoJSValue
/// - FromJSValue
/// - JSParameterType
///
/// Example:
/// ```rust
/// #[derive(JSType)]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
/// ```
#[proc_macro_derive(JSType)]
pub fn derive_js_bindings(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl rusty_js_core::IntoJSValue<JSEngineValue> for #name {
            fn into_js_value(self, context: &JSContext) -> JSEngineValue {
                rusty_js_core::Class::get::<Self>(context)
                    .map(|class| class.instance(self))
                    .unwrap_or_else(|| JSEngineValue::from((context.as_ref(), ())))
            }
        }

        impl rusty_js_core::FromJSValue<JSEngineValue> for #name {
            fn from_js_value(ctx: &JSContext, value: JSEngineValue) -> rusty_js_core::JSResult<Self> {
                let obj = rusty_js_core::JSObject::from_js_value(ctx, value)?;
                let instance = obj.borrow::<Self>()?;
                Ok(*instance)
            }
        }

        impl rusty_js_core::function::JSParameterType for #name {}
    };

    TokenStream::from(expanded)
}
