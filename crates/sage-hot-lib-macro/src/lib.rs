mod hot_lib;
mod util;

#[proc_macro_attribute]
pub fn hot_lib(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attr = syn::parse_macro_input!(attr as hot_lib::HotLibAttribute);
    let mut module = syn::parse_macro_input!(item as hot_lib::HotLibrary);
}