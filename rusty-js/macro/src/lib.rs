use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemStruct};

mod class;

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
