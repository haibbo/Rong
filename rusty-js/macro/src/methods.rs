use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ImplItemFn, ItemImpl, Lit, Meta};

/// # Property Attributes
///
/// - `configurable`: Property can be deleted and its attributes can be modified
/// - `enumerable`: Property shows up in enumerations of the object's properties
/// - The writable state is automatically determined by the presence of a setter
///
/// Configuration options for JavaScript method/property bindings.
///
/// # Property Types
///
/// Properties are automatically categorized as static or instance based on the presence
/// of a self receiver:
/// - Methods with no self receiver become static properties/methods
/// - Methods with self receiver become instance properties/methods
///
/// # Property Attributes
///
/// When defining a property with getter/setter, only the `enumerable` attribute needs to be
/// specified. This is because:
///
/// 1. All properties are automatically configurable (can be deleted and modified)
/// 2. The `enumerable` attribute controls property visibility:
///    - `true`: Property appears in `Object.keys()` and `for...in` loops
///    - `false`: Property is hidden from enumeration (default)
/// 3. Accessor properties (with getter/setter) don't use the `writable` attribute
///
/// ## Example
/// ```rust
/// #[methods]
/// impl Point {
///     // Public property - visible in Object.keys() and for...in
///     #[method(getter, enumerable, rename = "x")]
///     fn getx(&self) -> i32 {
///         self.x
///     }
///
///     // Internal property - hidden from enumeration
///     #[method(getter, rename = "y")]
///     fn gety(&self) -> i32 {
///         self.y
///     }
///
///     // Setter only needs the rename to match
///     #[method(setter, rename = "x")]
///     fn setx(&mut self, x: i32) {
///         self.x = x;
///     }
/// }
/// ```
#[derive(Default)]
struct MethodOpts {
    rename: Option<String>,
    getter: bool,
    setter: bool,
    enumerable: bool,
}

impl MethodOpts {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut opts = MethodOpts::default();

        for attr in attrs {
            if !attr.path().is_ident("method") {
                continue;
            }

            if let Meta::List(list) = &attr.meta {
                for nested in &list.parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                )? {
                    match nested {
                        Meta::NameValue(nv) if nv.path.is_ident("rename") => {
                            if let Expr::Lit(expr_lit) = &nv.value {
                                if let Lit::Str(s) = &expr_lit.lit {
                                    opts.rename = Some(s.value());
                                }
                            }
                        }
                        Meta::Path(path) if path.is_ident("getter") => {
                            opts.getter = true;
                        }
                        Meta::Path(path) if path.is_ident("setter") => {
                            opts.setter = true;
                        }
                        Meta::Path(path) if path.is_ident("enumerable") => {
                            opts.enumerable = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(opts)
    }
}

/// Process method attributes and generate JavaScript bindings
pub fn methods_impl(input: &ItemImpl, methods: &[ImplItemFn]) -> syn::Result<TokenStream> {
    let impl_type = &input.self_ty;
    let mut instance_methods = Vec::new();
    let mut static_methods = Vec::new();
    let mut constructor = None;

    // Type alias for property definition tuple
    type PropertyDefinition = (Option<TokenStream>, Option<TokenStream>, bool);

    // Separate maps for instance and static properties
    let mut instance_properties: std::collections::HashMap<String, PropertyDefinition> =
        std::collections::HashMap::new();
    let mut static_properties: std::collections::HashMap<String, PropertyDefinition> =
        std::collections::HashMap::new();

    for method in methods {
        let opts = MethodOpts::from_attrs(&method.attrs)?;
        let method_name = &method.sig.ident;
        let js_name = opts.rename.unwrap_or_else(|| method_name.to_string());

        // Check if this is a constructor
        if method.attrs.iter().any(|attr| {
            attr.path().is_ident("method")
                && attr
                    .meta
                    .require_list()
                    .ok()
                    .and_then(|list| list.parse_args::<Meta>().ok())
                    .map_or(false, |meta| meta.path().is_ident("constructor"))
        }) {
            constructor = Some(quote! {
                fn data_constructor() -> rusty_js::function::Constructor<rusty_js::JSEngineValue> {
                    rusty_js::function::Constructor::new(Self::#method_name)
                }
            });
            continue;
        }

        let params = &method.sig.inputs;
        let has_receiver = method.sig.receiver().is_some();

        if has_receiver {
            // Remove self parameter for instance methods
            let args: Vec<_> = params
                .iter()
                .skip(1)
                .map(|arg| {
                    if let syn::FnArg::Typed(pat_type) = arg {
                        (&*pat_type.pat, &*pat_type.ty)
                    } else {
                        unreachable!("Already skipped self receiver")
                    }
                })
                .collect();

            let (patterns, types): (Vec<_>, Vec<_>) = args.into_iter().unzip();

            // Handle instance methods with proper This/ThisMut mapping
            let (receiver_type, method_call) = if let Some(receiver) = method.sig.receiver() {
                if receiver.mutability.is_some() {
                    // For &mut self methods, use ThisMut and map to Self::method_name
                    (
                        quote! { mut this: rusty_js::function::ThisMut<#impl_type> },
                        quote! { Self::#method_name(&mut *this, #(#patterns),*) },
                    )
                } else {
                    // For &self methods, use This and map to Self::method_name
                    (
                        quote! { this: rusty_js::function::This<#impl_type> },
                        quote! { Self::#method_name(&*this, #(#patterns),*) },
                    )
                }
            } else {
                unreachable!("Already checked has_receiver")
            };

            // Handle property getters/setters
            if opts.getter || opts.setter {
                let func = quote! {
                    class.new_func(move |#receiver_type #(, #patterns: #types)*| {
                        #method_call
                    })
                };

                let entry = instance_properties
                    .entry(js_name.clone())
                    .or_insert_with(|| (None, None, opts.enumerable));

                if opts.getter {
                    entry.0 = Some(func);
                } else {
                    entry.1 = Some(func);
                }
                entry.2 |= opts.enumerable;
            } else {
                // Handle regular instance methods
                instance_methods.push(quote! {
                    class.method(
                        #js_name,
                        move |#receiver_type, #(#patterns: #types),*| {
                            #method_call
                        }
                    );
                });
            }
        } else {
            let args: Vec<_> = params
                .iter()
                .map(|arg| {
                    if let syn::FnArg::Typed(pat_type) = arg {
                        (&*pat_type.pat, &*pat_type.ty)
                    } else {
                        unreachable!("Static methods don't have self receiver")
                    }
                })
                .collect();

            let (patterns, types): (Vec<_>, Vec<_>) = args.into_iter().unzip();

            // Handle static property accessors or regular static methods
            if opts.getter || opts.setter {
                let func = quote! {
                    class.new_func(move |#(#patterns: #types),*| {
                        Self::#method_name(#(#patterns),*)
                    })
                };

                let entry = static_properties
                    .entry(js_name.clone())
                    .or_insert_with(|| (None, None, opts.enumerable));

                if opts.getter {
                    entry.0 = Some(func);
                } else {
                    entry.1 = Some(func);
                }
                entry.2 |= opts.enumerable;
            } else {
                // Handle regular static method
                static_methods.push(quote! {
                    class.static_method(
                        #js_name,
                        move |#(#patterns: #types),*| {
                            Self::#method_name(#(#patterns),*)
                        }
                    );
                });
            }
        }
    }

    let constructor = constructor.unwrap_or_else(|| {
        quote! {
            fn data_constructor() -> rusty_js::function::Constructor<rusty_js::JSEngineValue> {
                rusty_js::function::Constructor::new(|_: ()| panic!("No constructor defined"))
            }
        }
    });

    // Generate instance property definitions
    for (name, (getter, setter, enumerable)) in instance_properties {
        let mut parts = Vec::new();

        // First add accessors
        if let Some(getter) = getter {
            parts.push(quote! { .getter(#getter) });
        }
        if let Some(ref setter) = setter {
            parts.push(quote! { .setter(#setter) });
        }

        // Always set configurable by default
        parts.push(quote! { .configurable() });

        // Set enumerable if specified
        if enumerable {
            parts.push(quote! { .enumerable() });
        }

        let property = quote! {
            class.property(#name, |builder| builder #(#parts)*);
        };

        instance_methods.push(property);
    }

    // Generate static property definitions
    for (name, (getter, setter, enumerable)) in static_properties {
        let mut parts = Vec::new();

        // First add accessors
        if let Some(getter) = getter {
            parts.push(quote! { .getter(#getter) });
        }
        if let Some(ref setter) = setter {
            parts.push(quote! { .setter(#setter) });
        }

        // Always set configurable by default
        parts.push(quote! { .configurable() });

        // Set enumerable if specified
        if enumerable {
            parts.push(quote! { .enumerable() });
        }

        static_methods.push(quote! {
            class.static_property(#name, |builder| builder #(#parts)*);
        });
    }

    let output = quote! {
        impl rusty_js::JSClass<rusty_js::JSEngineValue> for #impl_type {
            const NAME: &'static str = Self::JS_CLASS_NAME;

            #constructor

            fn class_setup(class: &rusty_js::ClassSetup<rusty_js::JSEngineValue>) {
                #(#instance_methods)*
                #(#static_methods)*
            }
        }
    };

    // println!("Generated code:\n{}", output.to_string());
    Ok(output)
}
