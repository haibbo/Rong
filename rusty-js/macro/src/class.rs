use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, Lit, Meta};

/// Options for the class macro (currently unused but kept for future expansion)
#[derive(Default)]
pub struct ClassOpts {
    pub rename: Option<String>,
}

impl ClassOpts {
    pub fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut opts = ClassOpts::default();

        for attr in attrs {
            if !attr.path().is_ident("js_class") {
                continue;
            }

            if let Meta::List(list) = &attr.meta {
                for nested in list.parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                )? {
                    if let Meta::NameValue(nv) = nested {
                        if nv.path.is_ident("rename") {
                            if let Expr::Lit(expr_lit) = nv.value {
                                if let Lit::Str(s) = expr_lit.lit {
                                    opts.rename = Some(s.value());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(opts)
    }

    pub fn get_class_name(&self, type_name: &str) -> String {
        self.rename.clone().unwrap_or_else(|| type_name.to_string())
    }
}

/// Main implementation of the class macro
pub fn class_impl(input: &DeriveInput, opts: &ClassOpts) -> syn::Result<TokenStream> {
    let type_name = &input.ident;
    let js_name = opts.get_class_name(&type_name.to_string());
    let vis = &input.vis;
    let generics = &input.generics;
    let data = &input.data;

    // Filter out class attributes
    let filtered_attrs: Vec<_> = input
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("js_class"))
        .collect();

    // Rebuild type definition with filtered attributes
    let type_def = match data {
        syn::Data::Struct(s) => {
            let fields = &s.fields;
            quote! {
                #(#filtered_attrs)*
                #[derive(Clone, Copy)]
                #vis struct #type_name #generics #fields
            }
        }
        _ => return Err(syn::Error::new_spanned(input, "Only structs are supported")),
    };

    let expanded = quote! {
        #type_def

        impl #type_name {
            const JS_CLASS_NAME: &'static str = #js_name;
        }

        impl rusty_js::IntoJSValue<rusty_js::JSEngineValue> for #type_name {
            fn into_js_value(self, context: &rusty_js::JSContext) -> rusty_js::JSEngineValue {
                rusty_js::Class::get::<Self>(context)
                    .map(|class| class.instance(self))
                    .unwrap_or_else(|| rusty_js::JSEngineValue::from((context.as_ref(), ())))
            }
        }

        impl rusty_js::FromJSValue<rusty_js::JSEngineValue> for #type_name {
            fn from_js_value(ctx: &rusty_js::JSContext, value: rusty_js::JSEngineValue) -> rusty_js::JSResult<Self> {
                let obj = rusty_js::JSObject::from_js_value(ctx, value)?;
                let instance = obj.borrow::<Self>()?;
                Ok(*instance)
            }
        }

        impl rusty_js::function::JSParameterType for #type_name {}
    };

    Ok(expanded)
}
