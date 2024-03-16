#![feature(allocator_api)]

mod error;
mod lib_reload_events;
mod lib_reloader;

//TODO: Remove temporary logging file
mod log;

pub use error::HotReloaderError;
pub use lib_reload_events::{BlockReload, ChangedEvent, LibReloadNotifier, LibReloadObserver};
pub use lib_reloader::LibReloader;

pub use sage_hot_lib_macro::hot_lib;
