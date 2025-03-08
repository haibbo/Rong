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

        impl rusty_js::IntoJSValue<rusty_js::JSEngineValue> for #type_name {
            fn into_js_value(self, context: &rusty_js::JSContext) -> rusty_js::JSEngineValue {
                rusty_js::Class::get::<Self>(context)
                    .map(|class| class.instance(self).into_value())
                    .unwrap_or_else(|_| context.throw_error("Failed to make Class Instance").into_value())
            }
        }

        impl rusty_js::FromJSValue<rusty_js::JSEngineValue> for #type_name {
            fn from_js_value(ctx: &rusty_js::JSContext, value: rusty_js::JSEngineValue) -> rusty_js::JSResult<Self> {
                let obj = rusty_js::JSObject::from_js_value(ctx, value)?;
                let instance = obj.borrow::<Self>()?;
                Ok(instance.clone())
            }
        }

        impl rusty_js::function::JSParameterType for #type_name {}
    };

    Ok(expanded)
}
