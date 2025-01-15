#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use std::collections::HashSet;

use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn, Lit, Meta, MetaList, MetaNameValue, NestedMeta};

/// The `counter` attribute macro instruments a function with a metrics counter,
/// allowing you to measure how many times a function is called.
///
/// # Parameters
///
/// * `measurement`: The name of the measurement under which the count will be recorded (required).
/// * `tags`: An optional comma-separated list of key-value tuples for tagging the measurement,
///     such as `tags(key1 = "value1", key2 = "value2")`. The function name (`fn_name`) is
///     automatically added as a tag, so there is no need to include it manually. Tag keys must
///     be unique.
///
/// ## Examples
///
/// Create counter with tags.
///
/// ```ignore
/// use metricus_macros::counter;
///
/// #[counter(measurement = "counters", tags(key1 = "value1", key2 = "value2"))]
/// fn my_function_with_tags() {
///     // function body
/// }
/// ```
/// In this example, each call to `my_function_with_tags` increments a counter with the measurement name
/// "counters" and tagged with the environment. The function name is automatically tagged.
///
/// Create counter without tags.
///
/// ```ignore
/// use metricus_macros::counter;
///
/// #[counter(measurement = "counters")]
/// fn my_function_without_tags() {
///     // function body
/// }
/// ```
/// Here, each call to `my_function_without_tags` increments a counter with the measurement name
/// "counters". Only the function name is tagged automatically, since no additional tags were provided.
#[proc_macro_attribute]
pub fn counter(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;

    // initialize variables to hold parsed values
    let mut measurement = None;
    let mut tags = Vec::new();

    // auto include method name
    let method_name = fn_name.to_string();
    tags.push(("fn_name".to_string(), method_name));

    // keys must be unique
    let keys: HashSet<String> = tags.iter().map(|(k, _)| k).cloned().collect();
    assert_eq!(keys.len(), tags.len(), "must include unique tag keys");

    // Parse attributes for measurement and tags
    for arg in args {
        match arg {
            NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                ref path,
                lit: Lit::Str(ref value),
                ..
            })) if path.is_ident("measurement") => {
                measurement = Some(value.value());
            }
            NestedMeta::Meta(Meta::List(MetaList {
                ref path, ref nested, ..
            })) if path.is_ident("tags") => {
                for meta in nested {
                    if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                        path,
                        lit: Lit::Str(value),
                        ..
                    })) = meta
                    {
                        tags.push((path.get_ident().unwrap().to_string(), value.value()));
                    } else {
                        return TokenStream::from(
                            syn::Error::new_spanned(meta, "Expected a name-value pair for tags").to_compile_error(),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // Ensure consistent ordering of tags
    tags.sort_unstable_by(|(k1, _), (k2, _)| k1.cmp(k2));

    let tags: Vec<(&str, &str)> = tags.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let tags = tags.into_iter().map(|(k, v)| {
        // Directly quote each tuple
        quote! { (#k, #v) }
    });

    // Ensure measurement field is provided
    let measurement = match measurement {
        Some(measurement) => measurement,
        None => {
            return TokenStream::from(
                syn::Error::new_spanned(&input_fn, "Missing required 'measurement' field").to_compile_error(),
            )
        }
    };

    let measurement = measurement.as_str();

    // Reconstruct the original function and inject the counter

    let fn_body = &input_fn.block.stmts;
    let fn_vis = &input_fn.vis;
    let fn_unsafe = &input_fn.sig.unsafety;
    let fn_async = &input_fn.sig.asyncness;
    let fn_args = &input_fn.sig.inputs;
    let fn_output = &input_fn.sig.output;
    let fn_generics = &input_fn.sig.generics;
    let fn_where_clause = &input_fn.sig.generics.where_clause;
    let attrs = &input_fn.attrs;

    let gen = quote! {
        #(#attrs)*
        #fn_vis #fn_async #fn_unsafe fn #fn_name #fn_generics (#fn_args) #fn_output #fn_where_clause {

            static mut COUNTER: core::cell::LazyCell<core::cell::UnsafeCell<metricus::counter::Counter>> = core::cell::LazyCell::new(|| core::cell::UnsafeCell::new(metricus::counter::Counter::new(#measurement, &[ #(#tags),* ])));
            #[allow(static_mut_refs)]
            unsafe { metricus::counter::CounterOps::increment(&COUNTER); }

            #( #fn_body )*
        }
    };

    gen.into()
}
