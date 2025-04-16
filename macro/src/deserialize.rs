use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Attribute, Data, Expr, Fields, Lit, Meta, MetaNameValue};

/// Parse rename attribute to get JS field name
pub(crate) fn get_js_field_name(attrs: &[Attribute], rust_name: &str) -> String {
    for attr in attrs {
        if attr.path().is_ident("rename") {
            if let Meta::NameValue(MetaNameValue {
                value: Expr::Lit(expr_lit),
                ..
            }) = &attr.meta
            {
                if let Lit::Str(lit_str) = &expr_lit.lit {
                    return lit_str.value();
                }
            }
        }
    }
    rust_name.to_string()
}

pub(crate) fn impl_deserialize(input: syn::DeriveInput) -> TokenStream2 {
    let name = input.ident;

    // Get the fields from the struct
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("FromJSValue can only be derived for structs with named fields"),
        },
        _ => panic!("FromJSValue can only be derived for structs"),
    };

    // Generate field extractions
    let field_extractions = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let js_name = get_js_field_name(&field.attrs, &field_name.to_string());

        let js_name_lit = syn::LitStr::new(&js_name, field_name.span());

        // Check if field type is Option<T>
        let is_option = if let syn::Type::Path(ref type_path) = field_type {
            type_path
                .path
                .segments
                .last()
                .map(|seg| seg.ident == "Option")
                .unwrap_or(false)
        } else {
            false
        };

        if is_option {
            quote! {
                #field_name: match obj.get(#js_name_lit) {
                    Ok(val) => Some(val),
                    Err(rong_js::RongJSError::PropertyNotFound(_)) => None,
                    Err(e) => return Err(e),
                }
            }
        } else {
            quote! {
                #field_name: obj.get(#js_name_lit)?
            }
        }
    });

    let expanded = quote! {
        impl rong_js::FromJSValue<rong_js::JSEngineValue> for #name {
            fn from_js_value(ctx: &rong_js::JSContext, value: rong_js::JSEngineValue) -> rong_js::JSResult<Self> {
                let obj = rong_js::JSObject::from_js_value(ctx, value)?;
                Ok(Self {
                    #(#field_extractions,)*
                })
            }
        }

        impl rong_js::function::JSParameterType for #name {}
    };

    expanded
}
