use std::{
    fs,
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
    /// file name with platform-specific prefix and extension. (Except macos!)
    pub fn new(
        lib_dir: impl AsRef<Path>,
        lib_name: impl AsRef<str>,
        file_watch_debounce: Option<Duration>,
        loaded_lib_name_template: Option<String>,
    ) -> Result<Self, HotReloaderError> {
        // Find the target directory in which the build is happening and where we should find the library.
        let lib_dir = find_file_or_dir_in_parent_directories(lib_dir.as_ref())?;
        log::debug!("found lib dir at {lib_dir:?}"); //TODO: Replace logging.

        let load_counter = 0;

        // Determine the paths for the watched and loaded library files.
        let (watched_lib_file, loaded_lib_file) = watched_and_loaded_library_paths(
            &lib_dir,
            &lib_name,
            load_counter,
            &loaded_lib_name_template,
        );

        // Load the library and calculate its hash if it exists.
        let (lib_file_hash, lib) = if watched_lib_file.exists() {
            log::debug!("copying {watched_lib_file:?} -> {loaded_lib_file:?}"); //TODO: Replace logging.

            // Copy the library file to avoid file lock issues on some platforms (not implemented)
            fs::copy(&watched_lib_file, &loaded_lib_file)?;
            let hash = hash_file(&loaded_lib_file);
            (hash, Some(load_library(&loaded_lib_file)?))
        } else {
            log::debug!("library {watched_lib_file:?} does not yet exist");
            (0, None)
        };

        // Set up variables for tracking changes to the library file.
        let lib_file_hash = Arc::new(AtomicU32::new(lib_file_hash));
        let changed = Arc::new(AtomicBool::new(false));
        let file_change_subscribers = Arc::new(Mutex::new(Vec::new()));

        // Start watching the library file for changes.
        Self::watch(
            watched_lib_file.clone(),
            lib_file_hash.clone(),
            changed.clone(),
            file_change_subscribers.clone(),
            file_watch_debounce.unwrap_or_else(|| Duration::from_millis(500)),
        )?;

        // Initialize the `LibReloader` instance with the gathered information.
        let lib_loader = Self {
            load_counter,
            lib_dir,
            lib_name: lib_name.as_ref().to_string(),
            watched_lib_file,
            loaded_lib_file,
            lib,
            lib_file_hash,
            changed,
            file_change_subscribers,
            loaded_lib_name_template,
        };

        Ok(lib_loader)
    }
}

/// Try to find a file or directory that might be a relative path, such as `target/debug`,
/// by walking up the directories, starting from the current working directory (CWD). This
/// helps in finding the library when the app was started from a directory that is not the
/// project/workspace root.
fn find_file_or_dir_in_parent_directories(
    file: impl AsRef<Path>,
) -> Result<PathBuf, HotReloaderError> {
    // Convert the input to a PathBuf for easier manipulation.
    let mut file = file.as_ref().to_path_buf();

    // Check if the file doesn't exist and if it's a relative path.
    if !file.exists() && file.is_relative() {
        // Get the CWD.
        if let Ok(cwd) = std::env::current_dir() {
            // Start with the current directory as the parent directory.
            let mut parent_dir = Some(cwd.as_path());

            // Iterate up the directory tree.
            while let Some(dir) = parent_dir {
                // Check if the file exists in this directory.
                if dir.join(&file).exists() {
                    // Update the file path and break the loop.
                    file = dir.join(&file);
                    break;
                }
                // Move to the parent directory.
                parent_dir = dir.parent();
            }
        }
    }

    // Check if the file exists after walking up the directories.
    if file.exists() {
        // Return the found file path.
        Ok(file)
    } else {
        // Return an error.
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("file {file:?} does not exist"),
        )
        .into())
    }
}

/// Calculates the CRC32 hash of the contents of a file.
///
/// Reads the entire contents of the file and computes its CRC32 hash.
/// If the file cannot be read, the function returns a default hash value of 0.
///
/// # Arguments
/// * `f` - The path to the file whose contents will be hashed.
///
/// # Returns
/// The CRC32 hash of the file's contents as a `u32`.
fn hash_file(f: impl AsRef<Path>) -> u32 {
    fs::read(f.as_ref())
        .map(|content| crc32fast::hash(&content))
        .unwrap_or_default()
}

/// Loads a dynamic library at runtime.
///
/// Use `libloading` to load a dynamic library from the specified file path.
/// The function is marked as `unsafe` because loading arbitrary libraries at runtime can lead to
/// undefined behavior if the library is not compatible or if it contains malicious code.
///
/// # Arguments
/// * `lib_file` - The path to the dynamic library file to be loaded.
///
/// # Returns
/// A `Result` containing the loaded `Library` on success, or a `HotReloaderError` on failure.
fn load_library(lib_file: impl AsRef<Path>) -> Result<Library, HotReloaderError> {
    Ok(unsafe { Library::new(lib_file.as_ref()) }?)
}

/// Determines the file paths for the watched and loaded versions of a library.
///
/// # Arguments
/// * `lib_dir`: The directory containing the library.
/// * `lib_name`: The name of the library, without the platform-specific prefix and extension.
/// * `load_counter`: A counter used to differentiate between multiple loads of the same library.
/// * `loaded_lib_name_template`:   An optional template for the name of the loaded library.
///
/// # Returns
/// A tuple containing the paths to the watched and loaded library files.
fn watched_and_loaded_library_paths(
    lib_dir: impl AsRef<Path>,
    lib_name: impl AsRef<str>,
    load_counter: usize,
    loaded_lib_name_template: &Option<impl AsRef<str>>,
) -> (PathBuf, PathBuf) {
    // Convert the library directory to a Path reference.
    let lib_dir = &lib_dir.as_ref();

    // Determine the platform specific prefix and extension for the library file.
    #[cfg(target_os = "linux")]
    let (prefix, ext) = ("lib", "so");
    #[cfg(target_os = "windows")]
    let (prefix, ext) = ("", "dll");
    // Construct the full library name with the platform-specific prefix.
    let lib_name = format!("{prefix}{}", lib_name.as_ref());

    // Construct the path to the watched library file.
    let watched_lib_file = lib_dir.join(&lib_name).with_extension(ext);

    // Construct the file name for the loaded library using a template if provided.
    let loaded_lib_filename = match loaded_lib_name_template {
        Some(loaded_lib_name_template) => {
            let result = loaded_lib_name_template
                .as_ref()
                .replace("{lib_name}", &lib_name)
                .replace("{load_counter}", &load_counter.to_string())
                .replace("{pid}", &std::process::id().to_string());

            result
        }
        None => format!("{lib_name}-hot-{load_counter}"),
    };

    // Construct the path to the loaded library file.
    let loaded_lib_file = lib_dir.join(loaded_lib_filename).with_extension(ext);

    // Return the paths to the watched and loaded library files.
    (watched_lib_file, loaded_lib_file)
}
