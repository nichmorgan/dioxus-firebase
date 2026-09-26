//! One-shot Firebase App + Auth initialization.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::config::FirebaseOptions;
use crate::error::FirebaseError;
use crate::native;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::Acquire)
}

pub(crate) fn require_initialized() -> Result<(), FirebaseError> {
    if is_initialized() {
        Ok(())
    } else {
        Err(FirebaseError::NotInitialized)
    }
}

/// Initialize Firebase App (if needed) and Auth once.
///
/// The app supplies [`FirebaseOptions`] — never hardcode keys in this crate.
///
/// Double-init returns [`FirebaseError::AlreadyInitialized`] without calling
/// native code again. Desktop and web return
/// [`FirebaseError::UnsupportedPlatform`].
///
/// Uses compare-and-swap to claim the init slot so concurrent callers cannot
/// both reach native init. On failure the flag is released so retries work.
///
/// Call from `spawn_blocking` / a worker on mobile — the native host may hop
/// to the main thread and block until Firebase finishes initializing.
pub fn initialize(options: FirebaseOptions) -> Result<(), FirebaseError> {
    if INITIALIZED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(FirebaseError::AlreadyInitialized);
    }

    let result = options
        .validate()
        .and_then(|()| native::initialize(&options));
    if result.is_err() {
        INITIALIZED.store(false, Ordering::Release);
    }
    result
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn reset_initialized_for_test() {
    INITIALIZED.store(false, Ordering::Release);
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn mark_initialized_for_test() {
    INITIALIZED.store(true, Ordering::Release);
}
