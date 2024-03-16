use std::{
    borrow::BorrowMut,
    sync::{mpsc, Arc, Condvar, Mutex},
    time::Duration,
};

use crate::log;

/// Signals when the library has changed.
/// Needs to be public as it is used in the `hot_lib` macro.
#[derive(Clone)]
pub enum ChangedEvent {
    LibAboutToReload(BlockReload),
    LibReloaded,
}

impl std::fmt::Debug for ChangedEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LibAboutToReload(_) => write!(f, "LibAboutToReload"),
            Self::LibReloaded => write!(f, "LibReloaded"),
        }
    }
}

/// See [`LibReloadObserver::wait_for_about_to_reload`].
///
/// [`BlockReload`] is implemented using a simple counting scheme to track how many
/// tokens are floating around. If the number reaches 0 the update can continue.
#[derive(Debug)]
pub struct BlockReload {
    pub(crate) pending: Arc<(Mutex<usize>, Condvar)>,
}

impl Clone for BlockReload {
    fn clone(&self) -> Self {
        **(self.pending.0.lock().unwrap().borrow_mut()) += 1;
        Self {
            pending: self.pending.clone(),
        }
    }
}

impl Drop for BlockReload {
    fn drop(&mut self) {
        let (counter, cond) = &*self.pending;
        *counter.lock().unwrap() -= 1;
        cond.notify_one();
    }
}

/// The [`LibReloadObserver`] provides mechanisms to be notified about changes in a
/// dynamically loaded library. It offers two primary methods:
///
/// *   [`LibReloadObserver::wait_for_about_to_reload`]: This method signals that a reload
///     of the library is imminent, allowing you to perform any necessary preparations before
///     the old version is unloaded.
///
/// *   [`LibReloadObserver::wait_for_reload`]: This method indicates that the reload process
///     has finished, and the new library version is now loaded. You can use this notification
///     to restore state or perform actions specific to the updated library.
///
/// These methods can be used independently or in combination. A common use case involves
/// serializing state before a library update and then deserializing or migrating it after the
/// update is complete. Here's an example:
///
/// ```ignore
/// #[hot_module(dylib = "lib")]
/// mod hot_lib {
///     #[lib_change_subscription]
///     pub fn subscribe() -> hot_lib_reloader::LibReloadObserver { }
/// }
///
/// fn test() {
///     let lib_observer = hot_lib::subscribe();
///
///     /* ... */
///
///     // wait for reload to begin (at this point the  old version is still loaded)
///     let update_blocker = lib_observer.wait_for_about_to_reload();
///
///     /* do update preparations here, e.g. serialize state */
///
///     // drop the blocker to allow update
///     drop(update_blocker);
///
///     // wait for reload to be completed
///     lib_observer.wait_for_reload();
///
///     /* new lib version is loaded now so you can e.g. restore state */
/// }
/// ```
pub struct LibReloadObserver {
    // Needs to be public as it is used in the `hot_lib` macro.
    pub rx: mpsc::Receiver<ChangedEvent>,
}

impl LibReloadObserver {
    /// A call to this method will do a blocking wait until the watched library is
    /// about to change. It returns a [`BlockReload`] token. While this token is in
    /// scope you will prevent the pending update to proceed. This is useful for
    /// doing preparations for the update and while the old library version is still
    /// loaded. You can for example serialize state.
    pub fn wait_for_about_to_reload(&self) -> BlockReload {
        loop {
            match self.rx.recv() {
                Ok(ChangedEvent::LibAboutToReload(block)) => return block,
                Err(err) => {
                    panic!("LibReloadObserver failed to wait for event from reloader: {err}")
                }
                _ => continue,
            }
        }
    }

    /// Like [`Self::wait_for_about_to_reload`] but for a limited time. In case of a timeout return `None`.
    pub fn wait_for_about_to_reload_timeout(&self, timeout: Duration) -> Option<BlockReload> {
        loop {
            match self.rx.recv_timeout(timeout) {
                Ok(ChangedEvent::LibAboutToReload(block)) => return Some(block),
                Err(_) => return None,
                _ => continue,
            }
        }
    }

    /// Will do blocking wait until a new library version is loaded.
    pub fn wait_for_reload(&self) {
        loop {
            match self.rx.recv() {
                Ok(ChangedEvent::LibReloaded) => return,
                Err(err) => {
                    panic!("LibReloadObserver failed to wait for event from reloader: {err}")
                }
                _ => continue,
            }
        }
    }

    /// Like [`Self::wait_for_reload`] but for a limited time. In case of a timeout return `false`.
    pub fn wait_for_reload_timeout(&self, timeout: Duration) -> bool {
        loop {
            match self.rx.recv_timeout(timeout) {
                Ok(ChangedEvent::LibReloaded) => return true,
                Err(_) => return false,
                _ => continue,
            }
        }
    }
}

/// Needs to be public as it is used in the `hot_lib` macro.
#[derive(Default)]
pub struct LibReloadNotifier {
    subscribers: Arc<Mutex<Vec<mpsc::Sender<ChangedEvent>>>>,
}

impl LibReloadNotifier {
    /// Sends an event indicating the library is about to reload
    /// and waits for all [`BlockReload`] tokens to be dropped.
    ///
    /// It manages the synchronization of the library reloading
    /// process by using a conditional variable and a counter.
    /// The counter represents the number of [`BlockReload`] tokens
    /// that are still active. When a token is dropped, the counter
    /// is decremented, and once all tokens are dropped (counter
    /// reaches zero), the conditional variable is signaled to
    /// proceed with the library reload.
    ///
    /// Needs to be public as it is used in the `hot_lib` macro.
    pub fn send_about_to_reload_event_and_wait_for_blocks(&self) {
        // Create a shared state consisting of a counter and a condvar.
        // The counter is initialized to 1 to represent the initial state.
        let pending = Arc::new((Mutex::new(1), std::sync::Condvar::new()));

        // Create a `BlockReload` token with a reference to the shared state.
        let block = BlockReload {
            pending: pending.clone(),
        };

        // Notify observers that the library is about to reload by sending
        // the LibAboutToReload event along with the `BlockReload token`.
        self.notify(ChangedEvent::LibAboutToReload(block));

        // Unpack the shared state into the counter and the conditional variable.
        let (counter, cond) = &*pending;

        log::trace!(
            //TODO: Replace logging.
            "about-to-change library event, waiting for {}",
            counter.lock().unwrap()
        );

        // Wait until the counter reaches zero, indicating all tokens have been dropped.
        // The wait_while method releases the lock and waits for (*pending > 0) to be false.
        let _guard = cond
            .wait_while(counter.lock().unwrap(), |pending| {
                log::trace!(
                    //TODO: Replace logging.
                    "about-to-change library event, now waiting for {}",
                    *pending
                );
                *pending > 0
            })
            .unwrap();
    }

    /// Send a reloaded event.
    /// Needs to be public as it is used in the `hot_lib` macro.
    pub fn send_reloaded_event(&self) {
        self.notify(ChangedEvent::LibReloaded);
    }

    /// Sends a `ChangedEvent` to all active subscribers
    /// and removes any inactive subscribers.
    ///
    /// # Arguments
    /// * `evt`: The event to be sent to all subscribers.
    fn notify(&self, evt: ChangedEvent) {
        // Attempt to aquire a lock on the subscribers list.
        if let Ok(mut subscribers) = self.subscribers.try_lock() {
            // Get the initial number of subscribers
            let n = subscribers.len();
            log::trace!("sending {evt:?} to {n} subscribers"); //TODO: Replace logging.

            // Send the event to each subscriber and retain only active subscribers.
            subscribers.retain(|tx| tx.send(evt.clone()).is_ok());

            // Calculate the number of subscribers removed, log them.
            let removed = n - subscribers.len();
            if removed > 0 {
                log::debug!(
                    //TODO: Replace logging.
                    "removing {removed} subscriber{}",
                    if removed == 1 { "" } else { "s" }
                );
            }
        }
    }

    /// Subscribes to recieve notifications when the library has changed.
    pub fn subscribe(&mut self) -> LibReloadObserver {
        log::trace!("subscribe to lib change"); //TODO: Replace logging.
                                                // Create a new channel, aquire mutable lock on subscribers.
        let (tx, rx) = mpsc::channel();
        let mut subscribers = self.subscribers.lock().unwrap();
        // Add the sender to the list of subscribers, return the reciever inside a `LibReloadObserver`.
        subscribers.push(tx);
        LibReloadObserver { rx }
    }
}
