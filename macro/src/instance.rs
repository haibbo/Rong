use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

/// Main implementation of the object macro
pub fn class_instance_impl(input: &DeriveInput) -> syn::Result<TokenStream> {
    let type_name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;
    let data = &input.data;

    // Filter out object attributes
    let filtered_attrs: Vec<_> = input
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("js_export"))
        .collect();

    // Rebuild type definition with filtered attributes
    let type_def = match data {
        syn::Data::Struct(s) => {
            let fields = &s.fields;
            quote! {
                #(#filtered_attrs)*
                #[derive(Clone)]
                #vis struct #type_name #generics #fields
            }
        }
        _ => return Err(syn::Error::new_spanned(input, "Only structs are supported")),
    };

    let expanded = quote! {
        #type_def

        impl rong::IntoJSValue<rong::JSEngineValue> for #type_name {
            fn into_js_value(self, context: &rong::JSContext) -> rong::JSValue {
                rong::Class::get::<Self>(context)
                    .map(|class| class.instance(self).into_js_value())
                    .unwrap_or_else(|_| context.throw_error("Failed to make Class Instance"))
            }
        }

        impl rong::FromJSValue<rong::JSEngineValue> for #type_name {
            fn from_js_value(ctx: &rong::JSContext, value: rong::JSValue) -> rong::JSResult<Self> {
                let obj = rong::JSObject::from_js_value(ctx, value)?;
                let instance = obj.borrow::<Self>()?;
                // Some JS-exposed structs implement an inherent `clone()` method (e.g. `Response.prototype.clone()`).
                // Method-call syntax would prefer the inherent method over `Clone::clone`, which would break
                // internal "this" passing by returning a fresh value instead of a plain Rust clone.
                Ok(<Self as ::core::clone::Clone>::clone(&*instance))
            }
        }

        impl rong::function::JSParameterType for #type_name {}
    };

    Ok(expanded)
}
