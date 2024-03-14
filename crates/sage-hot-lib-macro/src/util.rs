use proc_macro2::Span;
use std::path::PathBuf;
use syn::{Error, ForeignItemFn, LitStr, Result};

/// Reads the contents of a Rust source file an dfinds the top level functions that have
/// - Public visibility.
/// - `#[no_mangle]` attribute.
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
            }
            _ => continue,
        }
    }
}
