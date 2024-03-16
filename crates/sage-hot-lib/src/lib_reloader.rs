use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use libloading::{Library, Symbol};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::new_debouncer;

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

    /// Subscribes to file change notifications.
    /// Public because it is utilized by the `hot_lib` macro.
    pub fn subscribe_to_file_changes(&mut self) -> mpsc::Receiver<()> {
        log::trace!("subscribe to file change");
        // Create a channel, lock the mutex.
        let (tx, rx) = mpsc::channel();
        let mut subscribers = self.file_change_subscribers.lock().unwrap();
        // Add the sender to the list of subscribers, return the reciever.
        subscribers.push(tx);
        rx
    }

    /// Checks if the watched library has changed and reloads it if necessary.
    ///
    /// Checks the `changed` flag to determine if the watched library has been modified.
    /// If the library has been changed, it reloads the library and returns `true` to indicate
    /// that an update has occurred. If the library has not changed, it returns `false`.
    ///
    /// # Errors
    /// Returns a `HotReloaderError` if the library fails to reload.
    ///
    /// # Returns
    /// `Ok(true)` if the library was successfully reloaded,
    /// `Ok(false)` if the library has not changed, or
    /// `Err(HotReloaderError)` if an error occurred during reloading.
    pub fn update(&mut self) -> Result<bool, HotReloaderError> {
        // Check if the library has changed using an atomic load with Acquire ordering for thread-safe reading.
        if !self.changed.load(Ordering::Acquire) {
            // If the library has not changed, return `Ok(false)` immediately.
            return Ok(false);
        }

        // If the library has changed, reset the `changed` flag to false using an atomic store
        // with Release ordering to ensure subsequent operations see this update.
        self.changed.store(false, Ordering::Release);

        // Attempt to reload the library. If an error occurs during reloading,
        // propagate the error to the caller using the `?` operator.
        self.reload()?;

        // If the library was successfully reloaded, return `Ok(true)`.
        Ok(true)
    }

    /// Reloads the library specified by `self.lib_file`.
    ///
    /// Closes the currently loaded library, if any, copies the new library file
    /// to a location where it can be loaded, and loads the copied library file.
    ///
    /// # Returns
    /// A `Result` indicating the success or failure of the reload operation. If successful,
    /// returns `Ok(())`. If the library cannot be reloaded, returns an `Err` with a `HotReloaderError`.
    fn reload(&mut self) -> Result<(), HotReloaderError> {
        let Self {
            load_counter,
            lib_dir,
            lib_name,
            lib,
            watched_lib_file,
            loaded_lib_file,
            loaded_lib_name_template,
            ..
        } = self;

        log::info!("reloading lib {watched_lib_file:?}");

        // If a library is currently loaded, close it and remove the file if it exists.
        if let Some(lib) = lib.take() {
            lib.close()?;
            if loaded_lib_file.exists() {
                let _ = fs::remove_file(&loaded_lib_file);
            }
        }

        // If the library file to watch exists, proceed with reloading.
        if watched_lib_file.exists() {
            *load_counter += 1; // Increment the load counter for the new library version.

            // Determine the paths for the watched and loaded library files.
            let (_, loaded_lib_file) = watched_and_loaded_library_paths(
                lib_dir,
                lib_name,
                *load_counter,
                loaded_lib_name_template,
            );

            // Copy the watched library file to the location for loading.
            log::trace!("copy {watched_lib_file:?} -> {loaded_lib_file:?}");
            fs::copy(watched_lib_file, &loaded_lib_file)?;

            // Store the hash of the loaded library file for change detection.
            self.lib_file_hash
                .store(hash_file(&loaded_lib_file), Ordering::Release);

            // Load the copied library file and store the handle.
            self.lib = Some(load_library(&loaded_lib_file)?);

            // Update the loaded library file path.
            self.loaded_lib_file = loaded_lib_file;
        } else {
            log::warn!("trying to reload library but it does not exist");
        }

        Ok(())
    }

    /// Watches a library file for changes and notifies subscribers when changes occur.
    ///
    /// # Arguments
    /// * `lib_file` - The path to the library file to watch.
    /// * `lib_file_hash` - An atomic value representing the current hash of the library file.
    /// * `changed` - An atomic boolean that is set to true when the library file changes.
    /// * `file_change_subscribers` - A list of subscribers to notify when the library file changes.
    /// * `debounce` - The debounce duration to use for file change events.
    ///
    /// # Returns
    /// A `Result` indicating success or failure.
    fn watch(
        lib_file: impl AsRef<Path>,
        lib_file_hash: Arc<AtomicU32>,
        changed: Arc<AtomicBool>,
        file_change_subscribers: Arc<Mutex<Vec<mpsc::Sender<()>>>>,
        debounce: Duration,
    ) -> Result<(), HotReloaderError> {
        // Convert the library file path to a `PathBuf` for easier manipulation.
        let lib_file = lib_file.as_ref().to_path_buf();
        log::info!("start watching changes of file {}", lib_file.display());

        // Spawn a new thread to watch for file changes.
        thread::spawn(move || {
            // Create a channel for receiving file change events.
            let (tx, rx) = mpsc::channel();

            // Create a debouncer for file change events.
            let mut debouncer =
                new_debouncer(debounce, None, tx).expect("creating notify debouncer");

            // Start watching the library for file changes.
            debouncer
                .watcher()
                .watch(&lib_file, RecursiveMode::NonRecursive)
                .expect("watch lib file");

            // Define a closure to handle change detection and notification.
            let signal_change = || {
                // Check if the file hash has changed or if a change is already pending.
                if hash_file(&lib_file) == lib_file_hash.load(Ordering::Acquire)
                    || changed.load(Ordering::Acquire)
                {
                    return false;
                }

                log::debug!("{lib_file:?} changed");

                // Set the changed flag to true.
                changed.store(true, Ordering::Release);

                // Notify all subscribers of the change.
                let subscribers = file_change_subscribers.lock().unwrap();
                log::trace!(
                    "sending ChangedEvent::LibFileChanged to {} subscribers",
                    subscribers.len()
                );
                for tx in &*subscribers {
                    let _ = tx.send(());
                }

                true
            };

            // Enter the event loop to listen for file change events.
            loop {
                match rx.recv() {
                    Err(_) => {
                        log::info!("file watcher channel closed");
                        break;
                    }
                    Ok(events) => {
                        let events = match events {
                            Err(errors) => {
                                log::error!("{} file watcher error!", errors.len());
                                for err in errors {
                                    log::error!("  {err}");
                                }
                                continue;
                            }
                            Ok(events) => events,
                        };

                        log::trace!("file change events: {events:?}");
                        let was_removed =
                            events
                                .iter()
                                .fold(false, |was_removed, event| match event.kind {
                                    notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                                        false
                                    }
                                    notify::EventKind::Remove(_) => true,
                                    _ => was_removed,
                                });

                        // If the file was removed, attempt to watch it again.
                        if was_removed || !lib_file.exists() {
                            log::debug!(
                                "{} was removed, trying to watch it again...",
                                lib_file.display()
                            );
                        }
                        loop {
                            if debouncer
                                .watcher()
                                .watch(&lib_file, RecursiveMode::NonRecursive)
                                .is_ok()
                            {
                                log::info!("watching {lib_file:?} again after removal");
                                signal_change();
                                break;
                            }
                            thread::sleep(Duration::from_millis(500));
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Get a pointer to a function or static variable by symbol name.
    /// Just a wrapper around `libloading::Library::get`.
    ///
    /// The `symbol` may not contain any null bytes, with the exception of
    /// the last byte. Providing a null-terminated `symbol` may help to
    /// avoid an allocation. The symbol is interpreted as is, no mangling.
    ///
    /// # Safety
    /// Users of this API must specify the correct type of the function or variable loaded.
    pub unsafe fn get_symbol<T>(&self, name: &[u8]) -> Result<Symbol<T>, HotReloaderError> {
        match &self.lib {
            None => Err(HotReloaderError::LibraryNotLoaded),
            Some(lib) => Ok(lib.get(name)?),
        }
    }

    /// Logging helper for the hot lib macro.
    /// TODO: Replace with Sage logging system.
    pub fn log_info(what: impl std::fmt::Display) {
        log::info!("{}", what);
    }
}

/// Deletes the currently loaded lib file if it exists.
impl Drop for LibReloader {
    fn drop(&mut self) {
        if self.loaded_lib_file.exists() {
            log::trace!("removing {:?}", self.loaded_lib_file);
            let _ = fs::remove_file(&self.loaded_lib_file);
        }
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
