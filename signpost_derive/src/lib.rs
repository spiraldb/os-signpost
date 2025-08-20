#![warn(missing_docs)]

//! Procedural macros for the `signpost` crate
//!
//! This crate provides macros for instrumenting Rust code with Apple's signpost
//! API for performance profiling.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, ExprLit, ItemFn, Lit, LitStr, Meta, MetaNameValue, Result,
};

/// Automatically instrument a function with signposts
///
/// # Usage
///
/// ```ignore
/// #[signpost]
/// fn my_function() {
///     // Automatically instrumented
/// }
///
/// #[signpost("Custom Message")]
/// async fn async_function() -> Result<(), Error> {
///     // Works with async functions and early returns with message
/// }
///
/// #[signpost(message="Data Processing")]
/// fn process_data() {
///     // Function with custom message
/// }
/// ```
#[proc_macro_attribute]
pub fn signpost(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as InstrumentArgs);
    let input_fn = parse_macro_input!(input as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_vis = &input_fn.vis;
    let fn_sig = &input_fn.sig;
    let fn_block = &input_fn.block;
    let fn_attrs = &input_fn.attrs;

    // Generate the signpost message
    let signpost_message = args.message;

    // Generate common signpost setup
    let signpost_setup = quote! {
        let __logger = signpost::global_logger();
        let __id = signpost::SignpostId::generate(__logger);
    };

    // Generate interval creation based on whether message is provided
    let interval_creation = if let Some(message) = signpost_message {
        quote! {
            let _interval = __logger.interval_with_message(__id, &format!("{}::{}", module_path!(), stringify!(#fn_name)), #message);
        }
    } else {
        quote! {
            let _interval = __logger.interval(__id, &format!("{}::{}", module_path!(), stringify!(#fn_name)));
        }
    };

    // Generate instrumented function
    let instrumented = if fn_sig.asyncness.is_some() {
        // Handle async functions
        quote! {
            #(#fn_attrs)*
            #fn_vis #fn_sig {
                async move {
                    #signpost_setup
                    #interval_creation
                    let __result = async move #fn_block.await;
                    __result
                }
                .await
            }
        }
    } else {
        // Handle sync functions
        quote! {
            #(#fn_attrs)*
            #fn_vis #fn_sig {
                #signpost_setup
                #interval_creation
                #fn_block
            }
        }
    };

    TokenStream::from(instrumented)
}

struct InstrumentArgs {
    message: Option<String>,
}

impl Parse for InstrumentArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let message = if input.is_empty() {
            None
        } else if input.peek(LitStr) {
            // Parse direct string literal: "message"
            Some(input.parse::<LitStr>()?.value())
        } else {
            // Parse named argument: message = "value"
            let meta: Meta = input.parse()?;
            match meta {
                Meta::NameValue(MetaNameValue { path, value, .. }) if path.is_ident("message") => {
                    match value {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) => Some(lit_str.value()),
                        _ => return Err(syn::Error::new_spanned(value, "Expected string literal")),
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Expected 'message = \"...\"'",
                    ))
                }
            }
        };

        Ok(InstrumentArgs { message })
    }
}
