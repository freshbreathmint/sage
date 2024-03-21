mod hot_module;
mod util;

/// Top level interface for the sage dynamic Rust library hot reloader.
#[proc_macro_attribute]
pub fn hot_lib(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let attr = syn::parse_macro_input!(attr as hot_module::HotModuleAttribute);
    let mut module = syn::parse_macro_input!(item as hot_module::HotModule);
    module.hot_mod_attr = Some(attr);

    (quote::quote!( #module )).into()
}
