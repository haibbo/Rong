use proc_macro2::TokenStream;
use quote::quote;
use syn::{ImplItemFn, ItemImpl, Meta};

/// Process method attributes and generate JavaScript bindings
pub fn methods_impl(input: &ItemImpl, methods: &[ImplItemFn]) -> syn::Result<TokenStream> {
    let impl_type = &input.self_ty;

    // Find constructor method
    let constructor = methods
        .iter()
        .find(|method| {
            method.attrs.iter().any(|attr| {
                attr.path().is_ident("method")
                    && attr
                        .meta
                        .require_list()
                        .ok()
                        .and_then(|list| list.parse_args::<Meta>().ok())
                        .map_or(false, |meta| meta.path().is_ident("constructor"))
            })
        })
        .map(|method| {
            let method_name = &method.sig.ident;
            quote! {
                fn data_constructor() -> rusty_js::function::Constructor<rusty_js::JSEngineValue> {
                    rusty_js::function::Constructor::new(Self::#method_name)
                }
            }
        })
        .unwrap_or_else(|| {
            quote! {
                fn data_constructor() -> rusty_js::function::Constructor<rusty_js::JSEngineValue> {
                    rusty_js::function::Constructor::new(|_: ()| panic!("No constructor defined"))
                }
            }
        });

    let output = quote! {
        impl rusty_js::JSClass<rusty_js::JSEngineValue> for #impl_type {
            const NAME: &'static str = Self::JS_CLASS_NAME;

            #constructor

            fn class_setup(_class: &rusty_js::ClassSetup<rusty_js::JSEngineValue>) {}
        }
    };

    // println!("Generated code:\n{}", output.to_string());
    Ok(output)
}
