mod hot_lib;

#[proc_macro_attribute]
pub fn hot_lib(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let args = syn::parse_macro_input!(attr as hot_lib::HotLibAttribute);
}
