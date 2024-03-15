use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32},
        mpsc, Arc, Mutex,
    },
    time::Duration,
};

use libloading::Library;

use crate::{error::HotReloaderError, log};

/// Manages a dynamic library (dylib) file, loads it using libloading::Library,
/// and provides access to it's symbols. When the library changes, `LibReloader`
/// is able to unload the old version and reload the new version through
/// `LibReloader::update`.
///
/// Note that the `LibReloader` itself will not actively update, i.e. does not
/// manage an update thread calling the update function. This is normally managed
/// by the `hot_module` macro that also manages the `LibReloadNotifier` notifications.
///
/// It can load symbols from the library with `LibReloader::get_symbol`.
pub struct LibReloader {
    load_counter: usize,
    lib_dir: PathBuf,
    lib_name: String,
    changed: Arc<AtomicBool>,
    lib: Option<Library>,
    watched_lib_file: PathBuf,
    loaded_lib_file: PathBuf,
    lib_file_hash: Arc<AtomicU32>,
    file_change_subscribers: Arc<Mutex<Vec<mpsc::Sender<()>>>>,
    loaded_lib_name_template: Option<String>,
}

impl LibReloader {
    /// Creates a LibReloader.
    /// `lib_dir` is expected to be the location where the library to use can be found.
    /// Probably `target/debug` normally. `lib_name` is the name of the library, not(!)
    /// the file name. It should normally be just the crate name of the cargo project
    /// you want to hot-reload. `LibReloader` will take care to figure out the actual
    /// file name with platform-specific prefix and extension. (But not really!)
    pub fn new(
        lib_dir: impl AsRef<Path>,
        lib_name: impl AsRef<str>,
        file_watch_debounce: Option<Duration>,
        loaded_lib_name_template: Option<String>,
    ) -> Result<Self, HotReloaderError> {
        // Find the target directory in which the build is happening and where we should find the library.
        let lib_dir = find_file_or_dir_in_parent_directories(lib_dir.as_ref())?;
        log::debug!("found lib dir at {lib_dir:?}");
    }
}

/// Try to find a file or directory that might be a relative path, such as `target/debug`,
/// by walking up the directories, starting from the current working directory (CWD). This
/// helps in finding the library when the app was started from a directory that is not the
/// project/workspace root.
fn find_file_or_dir_in_parent_directories(
    file: impl AsRef<Path>,
) -> Result<PathBuf, HotReloaderError> {
}
