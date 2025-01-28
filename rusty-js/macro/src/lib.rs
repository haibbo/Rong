use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl, ItemStruct};

mod class;
mod methods;

/// Example:
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
pub fn class(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the input struct
    let input = parse_macro_input!(item as ItemStruct);

    // Parse options (currently unused but kept for future expansion)
    let opts = class::ClassOpts::default();

    // Generate the implementations
    match class::class_impl(&input, &opts) {
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
                // Only include methods that have our #[method] attribute
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
