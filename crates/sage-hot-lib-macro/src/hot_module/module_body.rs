use quote::ToTokens;
use syn::{
    spanned::Spanned, token, Attribute, Error, ForeignItemFn, Ident, Item, ItemMacro, LitBool,
    LitStr, Macro, Result, Visibility,
};

use super::code_gen::{
    gen_hot_module_function_for, gen_lib_change_subscription_function, gen_lib_version_function,
};
use crate::util::read_functions_from_file;

/// Represents a hot-loaded module.
///
/// Structure is used to store information about the hot-loaded library,
/// including its visibility, identifier, items (such as functions and types), attributes, and
/// any specific hot-loading attributes defined using the `HotMOduleAttribute` structure.
///
/// # Fields
/// * `vis`:            The visibility of the hot library.
/// * `ident`:          The identifier of the hot library.
/// * `items`:          A vector of items contained within the hot library,
///                     such as functions, types, and constants.
/// * `attributes`:     A vector of attributes applied to the hot library,
///                     such as `#[no_mangle]` or `#[export_name]`
/// * `hot_lib_attr`:   An optional `HotModuleAttribute` structure that contains specific
///                     attributes related to the hot library, such as the name of the
///                     dynamic library and the debounce duration for file watch events.
pub(crate) struct HotModule {
    pub(crate) vis: Visibility,
    pub(crate) ident: Ident,
    pub(crate) items: Vec<Item>,
    pub(crate) attributes: Vec<Attribute>,
    pub(crate) hot_lib_attr: Option<super::HotModuleAttribute>,
}

impl syn::parse::Parse for HotModule {
    fn parse(stream: syn::parse::ParseStream) -> Result<Self> {
        // Parse the outer attributes of the module and store them.
        let attributes = Attribute::parse_outer(stream)?;

        // Parse the visibility of the module.
        // If no visibility is specified, default to Inherited.
        let vis = stream
            .parse::<Visibility>()
            .unwrap_or(Visibility::Inherited);

        // Parse and consume the `mod` keyword, which is expected to precede the module.
        stream.parse::<token::Mod>()?;

        // Parse the identifier of the module.
        let ident = stream.parse::<Ident>()?;

        // Get a new parse stream for the module body.
        let module_body_stream;
        syn::braced!(module_body_stream in stream);

        // Initialize an empty vector to store items inside the module.
        let mut items = Vec::new();

        // Iterate over and parse each item in the module body until there are no more.
        while !module_body_stream.is_empty() {
            // Parse the next item from the module body stream.
            let item = module_body_stream.parse::<Item>()?;

            // Match the parsed item to determine its type and handle it accordingly.
            match item {
                // Macro: hot_functions_from_file!()
                Item::Macro(ItemMacro {
                    mac: Macro { path, tokens, .. },
                    ..
                }) if path.is_ident("hot_functions_from_file") => {
                    // Extract the span.
                    let span = path.span();
                    // Create an iterator over the tokens provided to the macro.
                    let mut iter = tokens.into_iter();

                    // Get the filename.
                    let file_name = iter
                        .next()
                        .ok_or_else(|| {
                            Error::new(span, "expected path to file as a literal string")
                        })
                        .and_then(|t| syn::parse2::<LitStr>(t.into_token_stream()))?;

                    // Parse optional parameter `ignore_no_mangle = true`
                    let ignore_no_mangle = if let Some(tokens) = iter.next() {
                        match tokens {
                            // Check if the next token is a comma, indicating more parameters.
                            proc_macro2::TokenTree::Punct(p) if p.as_char() == ',' => {
                                // Expect the next token to be the identifier "ignore_no_mangle"
                                let ident = iter
                                    .next()
                                    .ok_or_else(|| Error::new(ident.span(), "expected ident"))
                                    .and_then(|t| syn::parse2::<Ident>(t.to_token_stream()))?;
                                if ident != "ignore_no_mangle" {
                                    return Err(Error::new(ident.span(), "unexpected input"));
                                }

                                // Expect an equals sign after the identifier.
                                iter.next()
                                    .ok_or_else(|| Error::new(ident.span(), "expected ="))
                                    .and_then(|t| syn::parse2::<token::Eq>(t.to_token_stream()))?;

                                // Expect a boolean value after the equals sign.
                                let val = iter
                                    .next()
                                    .ok_or_else(|| {
                                        Error::new(ident.span(), "expected boolean value")
                                    })
                                    .and_then(|t| syn::parse2::<LitBool>(t.to_token_stream()))?;
                                val.value()
                            }
                            // If the next token is not a comma, return an error.
                            other => {
                                return Err(Error::new(other.span(), "expected comma"));
                            }
                        }
                    } else {
                        // If there are no more tokens, set `ignore_no_mangle` to false.
                        false
                    };

                    // Read functions from the specified file.
                    let functions = read_functions_from_file(file_name, ignore_no_mangle)?;

                    // Iterate over each function and its span.
                    for (f, span) in functions {
                        // Generate a hot lib function for each function.
                        let f = gen_hot_module_function_for(f, span)?;

                        // Add the generated function the list of items in the `HotModule`.
                        items.push(Item::Fn(f));
                    }
                }

                // #[lib_change_subscription]
                Item::Fn(func)
                    if func
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("lib_change_subscription")) =>
                {
                    // Get the span of the function.
                    let span = func.span();

                    // Create a new `ForeignItemFn` based on the parsed function.
                    let f = ForeignItemFn {
                        attrs: Vec::new(),
                        vis: func.vis,
                        sig: func.sig,
                        semi_token: token::Semi::default(),
                    };

                    // Generate the actual function for the library change subscription.
                    let f = gen_lib_change_subscription_function(f, span)?;

                    // Add the generated function to the list of items in the `HotModule`.
                    items.push(Item::Fn(f));
                }

                // #[lib_version]
                Item::Fn(func)
                    if func
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("lib_version")) =>
                {
                    // Get the span of the function.
                    let span = func.span();

                    // Create a new `ForeignItemFn` based on the parsed function.
                    let f = ForeignItemFn {
                        attrs: Vec::new(),
                        vis: func.vis,
                        sig: func.sig,
                        semi_token: token::Semi::default(),
                    };

                    // Generate the actual function for the library version.
                    let f = gen_lib_version_function(f, span)?;

                    // Add the generated function to the list of items in the `HotModule`.
                    items.push(Item::Fn(f));
                }

                //TODO: #[lib_updated]

                //TODO: #[hot_function]

                //TODO: #[hot_functions]

                // Push the item as it is.
                item => items.push(item),
            };
        }

        // Construct a new `HotModule` with the parsed quality.
        Ok(Self {
            vis,
            ident,
            items,
            attributes,
            hot_lib_attr: None,
        })
    }
}
