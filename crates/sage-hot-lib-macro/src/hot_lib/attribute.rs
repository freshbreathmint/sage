use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, Error, Expr, ExprAssign, ExprLit,
    ExprPath, Ident, Lit, LitInt, Path, Result,
};

/// Represents the attributes of a hot library.
pub(crate) struct HotLibAttribute {
    pub(crate) lib_name: Expr,
    pub(crate) lib_dir: Expr,
    pub(crate) file_watch_debounce_ms: LitInt,
    pub(crate) crate_name: Path,
    pub(crate) loaded_lib_name_template: Expr,
}

/// Implement the `Parse` trait for `HotLibAttribute` to enable parsing.
impl syn::parse::Parse for HotLibAttribute {
    fn parse(stream: syn::parse::ParseStream) -> Result<Self> {
        // Initialize optional HotLibAttribute fields to `None`.
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

        // Return the parsed `HotLibAttribute`.
        Ok(HotLibAttribute {
            lib_name,
            lib_dir,
            file_watch_debounce_ms,
            crate_name,
            loaded_lib_name_template,
        })
    }
}
