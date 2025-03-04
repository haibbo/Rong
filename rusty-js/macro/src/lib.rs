use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemImpl};

mod class;
mod deserialize;
mod methods;

/// Expose a Rust struct as a JavaScript class.
///
/// This macro generates the necessary code to make a Rust struct usable as a JavaScript class,
/// including type conversions and class registration.
///
/// # Attributes
/// - `rename = "name"`: Use a different name for the class in JavaScript
///
/// # Generated Implementations
/// - `IntoJSValue<JSEngineValue>`
/// - `FromJSObj<JSEngineValue>`
/// - `JSParameterType`
///
/// # Example
/// ```ignore
/// use rusty_js_macro::js_class;
///
/// #[js_class(rename = "Point2D")]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
/// ```
#[proc_macro_attribute]
pub fn js_class(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attr2: TokenStream2 = attr.into();

    // Create a new class attribute with the original attribute parameters.
    // This is necessary because the original attribute is consumed during macro expansion,
    // but we need to parse it again in class_impl to extract options like rename.
    let class_attr = syn::parse_quote!(#[js_class(#attr2)]);

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

/// Define JavaScript methods and properties for a class.
///
/// This macro can only be applied to impl blocks and processes method definitions
/// marked with `#[js_method]`. Methods can be exposed as:
/// - Regular methods
/// - Property getters/setters
/// - Static methods/properties
/// - Async methods (automatically converted to JavaScript Promises)
///
/// # Method Types
/// - Instance methods: Take `&self` or `&mut self`
/// - Static methods: No self parameter
/// - Constructors: Marked with `#[js_method(constructor)]`
/// - Async methods: Methods marked with `async` keyword
///
/// # Example
/// ```ignore
/// use rusty_js_macro::{js_class, js_method, js_methods};
///
/// #[js_class]
/// struct Point {
///     x: i32,
///     y: i32,
/// }
///
/// #[js_methods]
/// impl Point {
///     // Constructor
///     #[js_method(constructor)]
///     fn new(x: i32, y: i32) -> Self {
///         Self { x, y }
///     }
///
///     // Instance property
///     #[js_method(getter, enumerable)]
///     fn x(&self) -> i32 { self.x }
///
///     // Static method
///     #[js_method]
///     fn create(x: i32, y: i32) -> Self {
///         Self { x, y }
///     }
///
///     // Async instance method
///     #[js_method]
///     async fn move_by_async(&mut self, dx: i32, dy: i32) {
///         // Async operation
///         self.x += dx;
///         self.y += dy;
///     }
///
///     // Async static method
///     #[js_method]
///     async fn create_async(x: i32, y: i32) -> Self {
///         // Async operation
///         Self { x, y }
///     }
/// }
/// ```
///
/// # Async Methods
/// Async methods are automatically converted to JavaScript Promises:
/// - Rust async methods become JavaScript async functions
/// - Return values are wrapped in Promises
/// - Can be used with JavaScript `async/await` syntax
/// - Support both instance and static methods
/// - Can be used as property getters/setters
///
/// JavaScript usage:
/// ```javascript
/// // Using async instance method
/// let point = new Point(1, 2);
/// await point.moveByAsync(10, 20);
///
/// // Using async static method
/// let newPoint = await Point.createAsync(5, 6);
/// ```
#[proc_macro_attribute]
pub fn js_methods(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // First try to parse as impl block
    let result = syn::parse::<ItemImpl>(item.clone());

    // Return error if not an impl block
    if result.is_err() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[js_methods] can only be used on impl blocks",
        )
        .to_compile_error()
        .into();
    }

    let input = result.unwrap();

    // Process methods as before
    let methods: Vec<_> = input
        .items
        .iter()
        .filter_map(|item| {
            if let syn::ImplItem::Fn(method) = item {
                if method
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("js_method"))
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

/// Configure how a Rust method is exposed to JavaScript.
///
/// This attribute can only be applied to methods, not to impl blocks.
/// For impl blocks, use `#[js_methods]` instead.
///
/// This attribute configures the behavior of individual methods when they are
/// exposed to JavaScript. It supports various options for controlling how the
/// method appears and behaves in JavaScript.
///
/// # Options
/// - `getter`: Expose as a property getter
/// - `setter`: Expose as a property setter
/// - `enumerable`: Make the property visible in enumerations
/// - `rename = "name"`: Use a different name in JavaScript
/// - `constructor`: Mark as the class constructor
///
/// # Property Attributes
/// - All properties are configurable by default
/// - Properties are non-enumerable by default
/// - Writable state is determined by the presence of a setter
///
/// # Examples
/// ```ignore
/// use rusty_js_macro::{js_class, js_method, js_methods};
///
/// #[js_class]
/// struct MyClass {
///     value: i32,
/// }
///
/// #[js_methods]  // Use js_methods for impl block
/// impl MyClass {
///     // Constructor
///     #[js_method(constructor)]
///     fn new() -> Self { Self { value: 0 } }
///
///     // Public property with custom name
///     #[js_method(getter, enumerable, rename = "value")]
///     fn get_value(&self) -> i32 { self.value }
///
///     // Regular method
///     #[js_method(rename = "calculateTotal")]
///     fn calc_total(&self) -> i32 { self.value * 2 }
/// }
/// ```
#[proc_macro_attribute]
pub fn js_method(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Try to parse as impl block to check for misuse
    if syn::parse::<ItemImpl>(item.clone()).is_ok() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "Use #[js_methods] for impl blocks, not #[js_method]",
        )
        .to_compile_error()
        .into();
    }

    // Just pass through the original item if it's not an impl block
    item
}

/// Derive macro for implementing deserialization from JavaScript values to Rust structs.
///
/// This macro automatically implements the `FromJSObj` trait for a struct, allowing it
/// to be deserialized from JavaScript objects. Fields can be renamed using the `rename`
/// attribute to match different JavaScript property names.
///
/// # Attributes
/// - `rename = "name"`: Use a different name for the field in JavaScript
///
/// # Field Types
/// - Required fields must exist in the JavaScript object
/// - Optional fields should use `Option<T>` type
/// - All field types must implement `FromJSValue`
///
/// # Example
/// ```ignore
/// #[derive(FromJSObj)]
/// struct Person {
///     #[rename = "firstName"]
///     first_name: String,
///     #[rename = "lastName"]
///     last_name: String,
///     age: i32,
///     // Optional field
///     nickname: Option<String>,
/// }
/// ```
///
/// # JavaScript Usage
/// ```javascript
/// // This will successfully deserialize
/// const complete = {
///     firstName: "John",
///     lastName: "Doe",
///     age: 30,
///     nickname: "Johnny"
/// };
///
/// // This will fail because required field 'age' is missing
/// const incomplete = {
///     firstName: "John",
///     lastName: "Doe"
/// };
/// ```
#[proc_macro_derive(FromJSObj, attributes(rename))]
pub fn derive_from_js_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(deserialize::impl_deserialize(input))
}
