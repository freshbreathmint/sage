use syn::{punctuated::Punctuated, token::Comma, Expr, ExprPath, Ident, LitInt, Path, Result};

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

        // Helper function to check if an expression is an identifier.
        fn expr_is_ident<I>(expr: &Expr, ident: &I) -> bool
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
