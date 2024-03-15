use proc_macro2::Span;
use syn::{FnArg, ForeignItemFn, Item, ItemFn, LitByteStr, LitStr, Result, Visibility};

use crate::util::ident_from_pat;

/// Generates a hot-loading wrapper function for a given module function.
///
/// Takes a `ForeignItemFn` and a `Span`, generates an `ItemFn` that acts as a wrapper
/// for the original function, enabling hot-loading of the library at runtime.
///
/// # Arguments
///
/// * `lib_function`:   A `ForeignItemFn` representing the foreign library function to wrap.
/// * `span`:           A `Span` representing the source code location for error reporting.
///
/// # Returns
///
/// A `Result<ItemFn>` containing the generated wrapper function if successful,
/// or an error if the generation fails.
///
/// # Errors
///
/// May return an error if:
/// - The input function has a receiver / self type, which is not supported for exported library functions.
/// - There is an issue with symbol loading from the library at runtime.
pub(crate) fn gen_hot_module_function_for(
    lib_function: ForeignItemFn,
    span: Span,
) -> Result<ItemFn> {
    // Destructure the `lib_function` to extract it's signature.
    let ForeignItemFn { sig, .. } = lib_function;

    // Get the identifier of the function from it's signature.
    let fun_ident = &sig.ident;

    // Create a null terminated byte string for the function name.
    // This is required.
    let symbol_name = {
        let mut symbol_name = fun_ident.to_string().into_bytes();
        symbol_name.push(b'\0');
        LitByteStr::new(&symbol_name, Span::call_site())
    };

    // Get the return type of the function from it's signature.
    let ret_type = &sig.output;

    // Initialize vectors to store the input types and names.
    let mut input_types = Vec::new();
    let mut input_names = Vec::new();

    // Iterate over the function's input arguments.
    for arg in &sig.inputs {
        match arg {
            // Print a warning if the function has a receiver (self) type.
            FnArg::Receiver(_) => {
                eprintln!("warning: exported library name has receiver / self type");
                continue;
            }
            // For regular typed arguments, extract the type and name.
            FnArg::Typed(typed) => {
                input_types.push(typed.ty.clone());
                input_names.push(ident_from_pat(&typed.pat, &sig.ident, span)?);
            }
        }
    }

    // Create an error message for symbol loading faliure.
    let err_msg_load_symbol = LitStr::new(
        &format!("Cannot load library function {}", sig.ident),
        Span::call_site(),
    );

    // Create the body of the function to be generated.
    let block = syn::parse_quote! {
        {
            let lib_loader = __lib_loader();
            let lib_loader = lib_loader.read().expect("lib loader RwLock read failed");
            let sym = unsafe {
                lib_loader
                    .get_symbol::<fn( #( #input_names ),* ) #ret_type >(#symbol_name)
                    .expect(#err_msg_load_symbol)
            };
            sym( #( #input_names ),* )
        }
    };

    // Create the `ItemFn` representing the generated function.
    let function = ItemFn {
        attrs: Vec::new(),
        vis: Visibility::Public(syn::token::Pub::default()),
        sig,
        block,
    };

    // Return the generated function.
    Ok(function)
}

/// Generates a function that subscribes to library changes.
///
/// Takes a foreign function declaration and a span,
/// generates a new function that subscribes to library changes.
/// Returns an error if the function generation fails.
///
/// # Arguments
/// * `f_decl`: A `ForeignItemFn` representing the foreign function declaration.
/// * `span`:   A `Span` representing the source code span.
///
/// # Returns
/// A `Result<ItemFn>` representing the generated function definition.
pub(crate) fn gen_lib_change_subscription_function(
    f_decl: ForeignItemFn,
    span: Span,
) -> Result<ItemFn> {
    // Destructure the `ForeignItemFn` to extract the signature, visibility, and attributes.
    let ForeignItemFn {
        sig, vis, attrs, ..
    } = f_decl;

    // Return an `ItemFn` representing the generated function definition.
    Ok(ItemFn {
        attrs,
        vis,
        sig,
        block: syn::parse_quote_spanned! {span=>
            {
                __lib_loader_subscription()
            }
        },
    })
}

/// Generates a function that returns the current version of the library.
///
/// Takes a foreign function declaration and a span,
/// generates a new function that returns the library version.
/// Returns an error if the function generation fails.
///
/// # Arguments
/// * `f_decl`: A `ForeignItemFn` representing the foreign function declaration.
/// * `span`:   A `Span` representing the source code span.
///
/// # Returns
/// A `Result<ItemFn>` representing the generated function definition.
pub(crate) fn gen_lib_version_function(f_decl: ForeignItemFn, span: Span) -> Result<ItemFn> {
    // Destructure the `ForeignItemFn` to extract the signature, visibility, and attributes.
    let ForeignItemFn {
        sig, vis, attrs, ..
    } = f_decl;

    // Return an `ItemFn` representing the generated function definition.
    Ok(ItemFn {
        attrs,
        vis,
        sig,
        block: syn::parse_quote_spanned! {span =>
            {
                VERSION.load(::std::sync::atomic::Ordering::Aquire)
            }
        },
    })
}

pub(crate) fn gen_lib_was_updated_function(f_decl: ForeignItemFn, span: Span) -> Result<ItemFn> {
    // Destructure the `ForeignItemFn` to extract the signature, visibility, and attributes.
    let ForeignItemFn {
        sig, vis, attrs, ..
    } = f_decl;

    // Return an `ItemFn` representing the generated function definition.
    Ok(ItemFn {
        attrs,
        vis,
        sig,
        block: syn::parse_quote_spanned! {span =>
            {
                WAS_UPDATED.swap(false,::std::sync::atomic::Ordering::AcqRel)
            }
        },
    })
}
