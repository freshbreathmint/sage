use proc_macro2::Span;
use syn::{
    Expr, FnArg, ForeignItemFn, ItemFn, LitByteStr, LitInt, LitStr, Path, Result, Visibility,
};

use crate::util::ident_from_pat;

/// Generates the ncessary items for dynamically loading a library
/// and handling file changes to trigger hot reloading.
///
/// This function creates several static variables and functions:
/// - `LIB_CHANGE_NOTIFIER`: A static variable to hold the library change notifier.
/// - `LIB_CHANGE_NOTIFIER_INIT`: Initialization control for the library change notifier.
/// - `__lib_notifier()`: A function to access or initialize the library change notifier.
/// - `__lib_loader_subscription()`: A function to subscribe to library reload events.
/// - `LIB_LOADER`: A static variable that stores a library loader object.
/// - `LIB_LOADER_INIT`: Initialization control for the library loader object.
/// - `VERSION`: A version counter for reloads.
/// - `WAS_UPDATED`: A flag for indicating if an update occurred.
/// - `__lib_loader()`: A function to access or initialize the library loader.
///
/// The function also spawns a new thread to listen for file change events and reload the library.
///
/// # Arguments
/// * `lib_dir`:                    Expression representing the directory containing the library.
/// * `lib_name`:                   Expression representing the name of the library.
/// * `file_watch_debounce_ms`:     Literal integer representing the debounce time in milliseconds for file change events.
/// * `crate_name` -                Path representing the name of the crate.
/// * `loaded_lib_name_template`:   Expression representing the template for the loaded library name.
/// * `span`:                       Span used for generating the code with proper source location information.
///
/// # Returns
/// A `TokenStream` representing the generated items for library loading and change notification.
///
/// # Errors
/// Returns an error if any part of the generation process fails.
pub(crate) fn generate_lib_loader_items(
    lib_dir: &Expr,
    lib_name: &Expr,
    file_watch_debounce_ms: &LitInt,
    crate_name: &Path,
    loaded_lib_name_template: &Expr,
    span: Span,
) -> Result<proc_macro2::TokenStream> {
    let result = quote::quote_spanned! {span=>
        // Global variables for library change notification:
        // Static variable to hold the library change notifier.
        static mut LIB_CHANGE_NOTIFIER: Option<::std::sync::Arc<::std::sync::RwLock<#crate_name::LibReloadNotifier>>> = None;
        // Initialization control for the library change notifier.
        static LIB_CHANGE_NOTIFIER_INIT: ::std::sync::Once = ::std::sync::Once::new();

        // Function to access or initialize the library change notifier.
        fn __lib_notifier() -> ::std::sync::Arc<::std::sync::RwLock<#crate_name::LibReloadNotifier>> {
            // Initialize the notifier once.
            LIB_CHANGE_NOTIFIER_INIT.call_once(|| {
                let notifier = ::std::sync::Arc::new(::std::sync::RwLock::new(Default::default()));
                // Safety: This block is guarded by Once and will be called only one time.
                unsafe {
                    use ::std::borrow::BorrowMut;
                    *LIB_CHANGE_NOTIFIER.borrow_mut() = Some(notifier);
                }
            });

            // Return the notifier.
            // Safety: `Once` ensures that the global is initialized before access.
            unsafe { LIB_CHANGE_NOTIFIER.as_ref().cloned().unwrap() }
        }

        // Function to subscribe to library reload events.
        fn __lib_loader_subscription() -> #crate_name::LibReloadObserver {
            // Ensure library loader is initialized.
            let _ = __lib_loader();
            // Subscribe to reload events and return the observer.
            __lib_notifier()
                .write()
                .expect("write lock notifier")
                .subscribe()
        }

        // Global variables for library loading:
        // Static variable that stores a library loader object.
        static mut LIB_LOADER: Option<::std::sync::Arc<::std::sync::RwLock<#crate_name::LibReloader>>> = None;
        // Initialization control for the library loader object.
        static LIB_LOADER_INIT: ::std::sync::Once = ::std::sync::Once::new();

        // Version counter for reloads.
        static VERSION: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
        // Flag for indicating if an update occurred.
        static WAS_UPDATED: ::std::sync::atomic::AtomicBool = ::std::sync::atomic::AtomicBool::new(false);

        // Function to access or initialize the library loader.
        fn __lib_loader() -> ::std::sync::Arc<::std::sync::RwLock<#crate_name::LibReloader>> {
            // Initialize the loader once.
            LIB_LOADER_INIT.call_once(|| {
                // Create a new library reloader with the specified parameters.
                let mut lib_loader = #crate_name::LibReloader::new(#lib_dir, #lib_name, Some(::std::time::Duration::from_millis(#file_watch_debounce_ms)), #loaded_lib_name_template)
                    .expect("failed to create hot reload loader");

                // Subscribe to file change events and recieve a channel to listen for changes.
                let change_rx = lib_loader.subscribe_to_file_changes();
                // Wrap the library folder in an `Arc<RwLock>` for thread-safe access and mutation
                let lib_loader = ::std::sync::Arc::new(::std::sync::RwLock::new(lib_loader));
                // Clone the `Arc` to use in the update thread.
                let lib_loader_for_update = lib_loader.clone();

                // Spawn a new thread to listen for file change events and reload the library.
                let _thread = ::std::thread::spawn(move || {
                    loop {
                        // Wait for a file change event.
                        if let Ok(()) = change_rx.recv() {
                            // Notify subscribers about the impending library reload.
                            __lib_notifier()
                                .read()
                                .expect("read lock notifier")
                                .send_about_to_reload_event_and_wait_for_blocks();

                            // Attempt to aquire a write lock on the library loader to perform the update.
                            let mut first_lock_attempt = None;
                            loop {
                                if let Ok(mut lib_loader) = lib_loader_for_update.try_write() {
                                    if let Some(first_lock_attempt) = first_lock_attempt {
                                        let duration: ::std::time::Duration = first_lock_attempt - ::std::time::Instant::now();
                                        #crate_name::LibReloader::log_info(&format!("...got write lock after {}ms!", duration.as_millis()));
                                    }
                                    // Perform the library update.
                                    let _ = !lib_loader.update().expect("hot lib update()");
                                    break;
                                }
                                // If the write lock cannot be aquired immediately, record the first attempt time and try again.
                                if first_lock_attempt.is_none() {
                                    first_lock_attempt = Some(::std::time::Instant::now());
                                    #crate_name::LibReloader::log_info("trying to get a write lock...");
                                }
                                // Sleep for a short duration before retrying to aquire the write lock.
                                ::std::thread::sleep(::std::time::Duration::from_millis(1));
                            }

                            // Increment the version counter and mark the library as updated.
                            VERSION.fetch_add(1, ::std::sync::atomic::Ordering::Release);
                            WAS_UPDATED.store(true, ::std::sync::atomic::Ordering::Release);

                            // Notify subscribers that the library has been reloaded.
                            __lib_notifier()
                                .read()
                                .expect("read lock notifier")
                                .send_reloaded_event();
                        }
                    }
                });

                // Store the library loader in the global variable for later access.
                // Safety: This block is protected by `Once` and will only be executed once.
                unsafe {
                    use ::std::borrow::BorrowMut;
                    *LIB_LOADER.borrow_mut() = Some(lib_loader);
                }
            });

            // Safety: Once runs before and initializes the global.
            unsafe { LIB_LOADER.as_ref().cloned().unwrap() }
        }
    };

    Ok(result)
}

/// Generates a hot-loading wrapper function for a given module function.
///
/// Takes a `ForeignItemFn` and a `Span`, generates an `ItemFn` that acts as a wrapper
/// for the original function, enabling hot-loading of the library at runtime.
///
/// # Arguments
///
/// * `lib_function`:   A `ForeignItemFn` representing the foreign library function to wrap.
/// * `span`:           A `Span` representing the source code location for error reporting.
///
/// # Returns
///
/// A `Result<ItemFn>` containing the generated wrapper function if successful,
/// or an error if the generation fails.
///
/// # Errors
///
/// May return an error if:
/// - The input function has a receiver / self type, which is not supported for exported library functions.
/// - There is an issue with symbol loading from the library at runtime.
pub(crate) fn gen_hot_module_function_for(
    lib_function: ForeignItemFn,
    span: Span,
) -> Result<ItemFn> {
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

    // Create an error message for symbol loading faliure.
    let err_msg_load_symbol = LitStr::new(
        &format!("Cannot load library function {}", sig.ident),
        Span::call_site(),
    );

    // Create the body of the function to be generated.
    let block = syn::parse_quote! {
        {
            let lib_loader = __lib_loader();
            let lib_loader = lib_loader.read().expect("lib loader RwLock read failed");
            let sym = unsafe {
                lib_loader
                    .get_symbol::<fn( #( #input_names ),* ) #ret_type >(#symbol_name)
                    .expect(#err_msg_load_symbol)
            };
            sym( #( #input_names ),* )
        }
    };

    // Create the `ItemFn` representing the generated function.
    let function = ItemFn {
        attrs: Vec::new(),
        vis: Visibility::Public(syn::token::Pub::default()),
        sig,
        block,
    };

    // Return the generated function.
    Ok(function)
}

/// Generates a function that subscribes to library changes.
///
/// Takes a foreign function declaration and a span,
/// generates a new function that subscribes to library changes.
/// Returns an error if the function generation fails.
///
/// # Arguments
/// * `f_decl`: A `ForeignItemFn` representing the foreign function declaration.
/// * `span`:   A `Span` representing the source code span.
///
/// # Returns
/// A `Result<ItemFn>` representing the generated function definition.
pub(crate) fn gen_lib_change_subscription_function(
    f_decl: ForeignItemFn,
    span: Span,
) -> Result<ItemFn> {
    // Destructure the `ForeignItemFn` to extract the signature, visibility, and attributes.
    let ForeignItemFn {
        sig, vis, attrs, ..
    } = f_decl;

    // Return an `ItemFn` representing the generated function definition.
    Ok(ItemFn {
        attrs,
        vis,
        sig,
        block: syn::parse_quote_spanned! {span=>
            {
                __lib_loader_subscription()
            }
        },
    })
}

/// Generates a function that returns the current version of the library.
///
/// Takes a foreign function declaration and a span,
/// generates a new function that returns the library version.
/// Returns an error if the function generation fails.
///
/// # Arguments
/// * `f_decl`: A `ForeignItemFn` representing the foreign function declaration.
/// * `span`:   A `Span` representing the source code span.
///
/// # Returns
/// A `Result<ItemFn>` representing the generated function definition.
pub(crate) fn gen_lib_version_function(f_decl: ForeignItemFn, span: Span) -> Result<ItemFn> {
    // Destructure the `ForeignItemFn` to extract the signature, visibility, and attributes.
    let ForeignItemFn {
        sig, vis, attrs, ..
    } = f_decl;

    // Return an `ItemFn` representing the generated function definition.
    Ok(ItemFn {
        attrs,
        vis,
        sig,
        block: syn::parse_quote_spanned! {span =>
            {
                VERSION.load(::std::sync::atomic::Ordering::Aquire)
            }
        },
    })
}

/// Generates a function that returns the update status of the library.
///
/// Takes a foreign function declaration and a span,
/// generates a new function that returns the update status of a library
/// Returns an error if the function generation fails.
///
/// # Arguments
/// * `f_decl`: A `ForeignItemFn` representing the foreign function declaration.
/// * `span`:   A `Span` representing the source code span.
///
/// # Returns
/// A `Result<ItemFn>` representing the generated function definition.
pub(crate) fn gen_lib_was_updated_function(f_decl: ForeignItemFn, span: Span) -> Result<ItemFn> {
    // Destructure the `ForeignItemFn` to extract the signature, visibility, and attributes.
    let ForeignItemFn {
        sig, vis, attrs, ..
    } = f_decl;

    // Return an `ItemFn` representing the generated function definition.
    Ok(ItemFn {
        attrs,
        vis,
        sig,
        block: syn::parse_quote_spanned! {span =>
            {
                WAS_UPDATED.swap(false,::std::sync::atomic::Ordering::AcqRel)
            }
        },
    })
}
