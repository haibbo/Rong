use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ImplItemFn, ItemImpl, Lit, Meta};

/// Method configuration options
#[derive(Default)]
struct MethodOpts {
    rename: Option<String>,
}

impl MethodOpts {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut opts = MethodOpts::default();

        for attr in attrs {
            if !attr.path().is_ident("method") {
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
}

/// Process method attributes and generate JavaScript bindings
pub fn methods_impl(input: &ItemImpl, methods: &[ImplItemFn]) -> syn::Result<TokenStream> {
    let impl_type = &input.self_ty;
    let mut instance_methods = Vec::new();
    let mut static_methods = Vec::new();
    let mut constructor = None;

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

        // Process regular methods
        let params = &method.sig.inputs;
        let is_instance_method = method.sig.receiver().is_some();

        if is_instance_method {
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
            
            instance_methods.push(quote! {
                class.method(#js_name, move |this: rusty_js::function::This<#impl_type>, #(#patterns: #types),*| this.#method_name(#(#patterns),*));
            });
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
            
            static_methods.push(quote! {
                class.static_method(#js_name, move |#(#patterns: #types),*| Self::#method_name(#(#patterns),*));
            });
        }
    }

    let constructor = constructor.unwrap_or_else(|| {
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

            fn class_setup(class: &rusty_js::ClassSetup<rusty_js::JSEngineValue>) {
                #(#instance_methods)*
                #(#static_methods)*
            }
        }
    };

    // println!("Generated code:\n{}", output.to_string());
    Ok(output)
}
