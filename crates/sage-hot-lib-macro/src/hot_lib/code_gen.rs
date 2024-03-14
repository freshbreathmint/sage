use proc_macro2::Span;
use syn::{FnArg, ForeignItemFn, ItemFn, LitByteStr};

pub(crate) fn gen_hot_lib_function_for(lib_function: ForeignItemFn, span: Span) -> Result<ItemFn> {
    // Destructure the `lib_function` to extract it's signature.
    let ForeignItemFn { sig, .. } = lib_function;

    // Get the identifier of the function from it's signature.
    let fun_ident = &sig.ident;

    // Create a null terminated byte string for the function name.
    // This is required.
    let symbol_name = {
        let mut symbol_name = fun_ident.to_string().into_bytes();
        symbol_name.push(b'\0');
        LitByteStr::new(&symbol_name, Span::call_site())
    };

    // Get the return type of the function from it's signature.
    let ret_type = &sig.output;

    // Initialize vectors to store the input types and names.
    let mut input_types = Vec::new();
    let mut input_names = Vec::new();

    // Iterate over the function's input arguments.
    for arg in &sig.inputs {
        match arg {
            // Print a warning if the function has a receiver (self) type.
            FnArg::Receiver(_) => {
                eprintln!("warning: exported library name has receiver / self type");
                continue;
            }
            // For regular typed arguments, extract the type and name.
            FnArg::Typed(typed) => {
                input_types.push(typed.ty.clone());
                input_names.push(ident_from_pat(&typed.pat, &sig.ident, span)?);
            }
        }
    }
}
