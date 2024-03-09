#[proc_macro_attribute]
pub fn test_macro(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Parse the input as a function.
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    // Get the name of the function.
    let fn_name = &input.sig.ident;

    // Get the function body block
    let body = &input.block;

    // Generate the new function with the test logging.
    let expanded = quote::quote! {
        fn #fn_name() {
            println!("Calling function: {}", stringify!(#fn_name));
            #body
        }
    };

    proc_macro::TokenStream::from(expanded)
}
