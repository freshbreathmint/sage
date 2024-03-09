#[proc_macro_attribute]
pub fn test_macro(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // Parse the input as a function.
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    // Get the name of the function.
    let fn_name = &input.sig.ident;

    // Generate the new function with the test logging.
    let expanded = quote::quote! {
        fn #fn_name() {
            println!("Calling function: {}", stringify!(#fn_name));
            #input
        }
    };

    proc_macro::TokenStream::from(expanded)
}
