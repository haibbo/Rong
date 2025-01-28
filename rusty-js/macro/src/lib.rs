use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemImpl};

mod class;
mod methods;

/// Attribute macro for creating JavaScript classes from Rust structs
///
/// # Example
/// ```rust
/// #[class]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
/// ```
///
/// This will implement:
/// - `IntoJSValue<JSEngineValue>`
/// - `FromJSValue<JSEngineValue>`
/// - `JSParameterType`
#[proc_macro_attribute]
pub fn class(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attr2: TokenStream2 = attr.into();

    // Create a new class attribute with the original attribute parameters.
    // This is necessary because the original attribute is consumed during macro expansion,
    // but we need to parse it again in class_impl to extract options like rename.
    let class_attr = syn::parse_quote!(#[class(#attr2)]);

    // Create a new DeriveInput with all original attributes plus the reconstructed class attribute
    let mut new_input = input.clone();
    new_input.attrs.push(class_attr);

    // Parse options from attributes
    let opts = match class::ClassOpts::from_attrs(&new_input.attrs) {
        Ok(opts) => opts,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    match class::class_impl(&new_input, &opts) {
        Ok(expanded) => expanded.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Attribute macro for method configuration
#[proc_macro_attribute]
pub fn method(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Just pass through the original item
    item
}

/// Attribute macro for implementing methods
#[proc_macro_attribute]
pub fn methods(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    let methods: Vec<_> = input
        .items
        .iter()
        .filter_map(|item| {
            if let syn::ImplItem::Fn(method) = item {
                if method
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("method"))
                {
                    Some(method.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let impl_tokens = match methods::methods_impl(&input, &methods) {
        Ok(tokens) => tokens,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };

    let expanded = quote! {
        #input

        #impl_tokens
    };

    TokenStream::from(expanded)
}
