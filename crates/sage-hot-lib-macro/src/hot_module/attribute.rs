use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, Error, Expr, ExprAssign, ExprLit,
    ExprPath, Ident, Lit, LitInt, Path, Result,
};

/// Represents the attributes of a hot-loaded module.
///
/// Structure is used to store the parsed attributes from a procedural macro input.
/// Each field corresponds to an attribute that can be specified in the macro.
///
/// # Fields
/// * `lib_name`:                   Expression representing the name of the dynamic library.
/// * `lib_dir`:                    Expression representing the directory of the dynamic library.
/// * `file_watch_debounce_ms`:     Literal integer representing the debounce duration.
/// * `crate_name`:                 A path representing the crate name associated with the dynamic library.
/// * `loaded_lib_name_template`:   An expression representing a template for generating the name
///                                 of the loaded library.
pub(crate) struct HotModuleAttribute {
    pub(crate) lib_name: Expr,
    pub(crate) lib_dir: Expr,
    pub(crate) file_watch_debounce_ms: LitInt,
    pub(crate) crate_name: Path,
    pub(crate) loaded_lib_name_template: Expr,
}

/// Implement the `Parse` trait for `HotModuleAttribute` to enable parsing.
///
/// Allows for the parsing of `HotModuleAttribute` structures from procedural macro input.
/// It expects a series of assignment expressions seperated by commas, with specific attribute
/// names (`dylib`, `lib_dir`, `file_watch_debounce`, `crate` and `loaded_lib_name_template`).
/// Each attribute is optional, but if provided, it must adhear to the expected value type.
///
/// # Attributes
/// * `dylib`:                      The name of the dynamic library.
/// * `lib_dir`:                    The directory where the dynamic library is located.
/// * `file_watch_debounce`:        The debounce duration (in milliseconds) for file watch events.
/// * `crate`:                      The crate name associated with the dynamic library.
/// * `loaded_lib_name_template`:   A template for generating the name of the loaded library.
///
/// # Errors
/// Returns an error if the input does not conform to the expected format,
/// or if the required attributes are missing or have incorrect types.
impl syn::parse::Parse for HotModuleAttribute {
    fn parse(stream: syn::parse::ParseStream) -> Result<Self> {
        // Initialize optional HotModuleAttribute fields to `None`.
        let mut lib_name = None;
        let mut lib_dir = None;
        let mut file_watch_debounce_ms = None;
        let mut crate_name = None;
        let mut loaded_lib_name_template = None;

        // Parse the token stream into a non-empty, comma seperated list of expressions.
        let args = Punctuated::<Expr, Comma>::parse_separated_nonempty(stream)?;

        /// Helper function to check if an expression is an identifier.
        /// This is used to identify and extract specific attributes from the procedural macro.
        fn expr_is_ident<I: ?Sized>(expr: &Expr, ident: &I) -> bool
        where
            Ident: PartialEq<I>,
        {
            // Checks if the expression is of type `Path` using pattern matching.
            if let Expr::Path(ExprPath { path, .. }) = expr {
                // Ensure `ident` type can be compared for equality.
                path.is_ident(ident) // Implicitly returns true if the last segment matches `ident`.
            } else {
                // If not a `Path`, return `false`.
                false
            }
        }

        // Iterate over each argument in the parsed arguments.
        for arg in args {
            // Match for arguments that are assignment expressions.
            match arg {
                // If the argument is an assignment expression, destructure to get the left and right sides.
                Expr::Assign(ExprAssign { left, right, .. }) => match *right {
                    // If the right side is a literal int, and the left side is ident: "file_watch_debounce"
                    // Update field with the value of the literal integer.
                    Expr::Lit(ExprLit {
                        lit: Lit::Int(lit), ..
                    }) if expr_is_ident(&left, "file_watch_debounce") => {
                        file_watch_debounce_ms = Some(lit.clone());
                        continue;
                    }

                    // If the left side is ident: "dylib", update the field.
                    expr if expr_is_ident(&left, "dylib") => {
                        lib_name = Some(expr);
                        continue;
                    }

                    // If the left side is ident: "lib_dir", update the field.
                    expr if expr_is_ident(&left, "lib_dir") => {
                        lib_dir = Some(expr);
                        continue;
                    }

                    // If the left side is ident: "crate", parse the right side as a string literal.
                    expr if expr_is_ident(&left, "crate") => {
                        // Get the span of the expression for error reporting.
                        let span = expr.span().clone();

                        // Nested `match` statements to extract and validate the string literal.
                        // The outer `match` checks if expression is a literal expression.
                        let s = match match expr {
                            Expr::Lit(ExprLit { lit, .. }) => lit,
                            // If the expression is not a literal expression, return an error.
                            _ => return Err(Error::new(left.span(), "unexpected expression type")),
                        } {
                            // The inner `match` checks if the literal is a string literal.
                            Lit::Str(s) => s,
                            // If the literal is not a string literal, return an error.
                            _ => return Err(Error::new(span, "unexpected expression type")),
                        };

                        // Parse the string literal as a `Path` and update the field.
                        crate_name = Some(s.parse::<Path>().clone()?);
                        continue;
                    }

                    // If the left side is ident: "loaded_lib_name_template", update the field.
                    expr if expr_is_ident(&left, "loaded_lib_name_template") => {
                        loaded_lib_name_template = Some(expr);
                        continue;
                    }

                    // If none of the above conditions are met, return an error.
                    _ => return Err(Error::new(left.span(), "unexpected attribute name")),
                },

                // If the argument is not an assignment expression, return an error.
                _ => return Err(Error::new(arg.span(), "unexpected input")),
            }
        }

        // Assign the `lib_name` or return an error if it doesn't exist.
        let lib_name = match lib_name {
            None => {
                return Err(Error::new(
                    stream.span(),
                    r#"missing field "name": add `name = "name_of_library""#,
                ))
            }
            Some(lib_name) => lib_name,
        };

        // Assign the `lib_dir` or set it to the debug/release build folder.
        let lib_dir = match lib_dir {
            None => {
                if cfg!(debug_assertions) {
                    syn::parse_quote! { concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug") }
                } else {
                    syn::parse_quote! { concat!(env!("CARGO_MANIFEST_DIR"), "/target/release") }
                }
            }
            Some(lib_dir) => lib_dir,
        };

        // Assign the `file_watch_debounce_ms` or default it to 500 milliseconds.
        let file_watch_debounce_ms = match file_watch_debounce_ms {
            None => LitInt::new("500", stream.span()),
            Some(file_watch_debounce_ms) => file_watch_debounce_ms,
        };

        // Assign the `crate_name` or default the path to ::sage_hot_lib
        let crate_name = match crate_name {
            None => syn::parse_quote! { ::sage_hot_lib },
            Some(crate_name) => crate_name,
        };

        // Assign the `loaded_lib_name_template` or default to `None`.
        let loaded_lib_name_template = match loaded_lib_name_template {
            None => syn::parse_quote! { Option::None },
            Some(loaded_lib_name_template) => {
                syn::parse_quote! { Some(#loaded_lib_name_template.to_string()) }
            }
        };

        // Return the parsed `HotModuleAttribute`.
        Ok(HotModuleAttribute {
            lib_name,
            lib_dir,
            file_watch_debounce_ms,
            crate_name,
            loaded_lib_name_template,
        })
    }
}
