use syn::{punctuated::Punctuated, token, Error, Result};

pub(crate) struct HotLibAttribute {
    pub(crate) lib_name: syn::Expr,
    pub(crate) lib_dir: syn::Expr,
    pub(crate) file_watch_debounce_ms: syn::LitInt,
    pub(crate) crate_name: syn::Path,
    pub(crate) loaded_lib_name_template: syn::Expr,
}

// impl syn::parse::Parse for HotLibAttribute {
//     fn parse(stream: syn::parse::ParseStream) -> Result<Self> {
//         // Get the comma seperated expressions.
//         let args = Punctuated::<syn::Expr, token::Comma>::parse_separated_nonempty(stream)?;

//         for arg in args {
//             match arg {
//                 _ => return Err(Error::new(arg.span(), "unexpected input")),
//             }
//         }
//     }
// }
