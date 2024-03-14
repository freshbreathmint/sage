use proc_macro2::Span;
use std::path::PathBuf;
use syn::{Error, ForeignItemFn, LitStr, Pat, Result};

/// Extracts the identifier from a pattern in a function argument.
///
/// Used to extract the identifier of a function argument from it's pattern.
///
/// # Arguments
///
/// * `pat`:        The pattern of the argument.
/// * `func_name`:  The identifier of the function, used for error reporting.
/// * `span`:       The span of the function signature, used for error reporting.
///
/// # Returns
///
/// * `Ok(syn::Ident)`: The extracted identifier if the pattern is a simple identifier.
/// * `Err(Error)`:     An error if the pattern cannot be converted to an identifier.
pub fn ident_from_pat(pat: &Pat, func_name: &proc_macro2::Ident, span: Span) -> Result<syn::Ident> {
    match pat {
        Pat::Ident(pat) => Ok(pat.ident.clone()),
        _ => Err(Error::new(
            span,
            format!(
                "generating call for lib function: signature of function {func_name} cannot be converted"
            ),
        )),
    }
}

/// Reads the contents of a Rust source file an dfinds the top level functions that have
/// * Public visibility.
/// * `#[no_mangle]` attribute.
///
/// Functions are converted into a [syn::ForeignItemFn] so that they
/// can serve as lib function declarations of the library reloader.
pub fn read_functions_from_file(
    file_name: LitStr,
    ignore_no_mangle: bool,
) -> Result<Vec<(ForeignItemFn, Span)>> {
    // Extract the span of the file name and convert it into a `PathBuf`.
    let span = file_name.span();
    let path: PathBuf = file_name.value().into();

    // Check if the file exists, if not, return an error.
    if !path.exists() {
        return Err(Error::new(
            span,
            "Could not find file {path:?}, Please specify the file path from the root directory.",
        ));
    }

    // Read the contents of the file into a string.
    let content = std::fs::read_to_string(&path)
        .map_err(|err| Error::new(span, format!("Error reading file {path:?}: {err}")))?;

    // Parse the file into an abstract syntax tree.
    let ast = syn::parse_file(&content)?;

    // Initialize an empty vector to store the functions.
    let mut functions = Vec::new();

    // Iterate over each item in the abstract syntax tree.
    for item in ast.items {
        match item {
            // If the item is a function, process it, otherwise continue.
            syn::Item::Fn(fun) => {
                // Check if the function is public; if not, skip to the next item.
                match fun.vis {
                    syn::Visibility::Public(_) => {}
                    _ => continue,
                }

                // Check for the `#[no_mangle]` attribute, if not present skip to the next item.
                if !ignore_no_mangle {
                    let no_mangle = fun
                        .attrs
                        .iter()
                        .filter_map(|attr| attr.path.get_ident())
                        .any(|ident| *ident == "no_mangle");

                    if !no_mangle {
                        continue;
                    };
                }

                // Convert the function into a `ForeignItemFn` with an empty attributes vector
                // and the same visibility and signature as the original function.
                let fun = ForeignItemFn {
                    attrs: Vec::new(),
                    vis: fun.vis,
                    sig: fun.sig,
                    semi_token: syn::token::Semi(span),
                };

                // Add the converted function and its span to the `functions` vector.
                functions.push((fun, span));
            }
            _ => continue,
        }
    }

    // Return the vector of functions as a `Result`.
    Ok(functions)
}
