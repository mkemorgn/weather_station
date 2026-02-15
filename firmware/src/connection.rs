use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use log::{info, warn};

pub struct ConnectionManager {
    backoff_secs: Arc<Mutex<u64>>,
    needs_reconnect: Arc<AtomicBool>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            backoff_secs: Arc::new(Mutex::new(1)),
            needs_reconnect: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn current_backoff(&self) -> u64 {
        match self.backoff_secs.lock() {
            Ok(lock) => *lock,
            Err(_) => {
                warn!("Backoff Mutex poisoned! Defaulting to 1s");
                1
            }
        }
    }

    /// Fails the current attempt, doubling the backoff time up to 30 seconds.
    pub fn fail(&self) {
        if let Ok(mut delay) = self.backoff_secs.lock() {
            *delay = std::cmp::min(*delay * 2, 30);
            warn!("Connection attempt failed. Next backoff: {}s", *delay);
        }
    }

    /// Resets the backoff to the initial state (1 second).
    pub fn reset(&self) {
        self.needs_reconnect.store(false, Ordering::SeqCst);
        if let Ok(mut delay) = self.backoff_secs.lock() {
            if *delay > 1 {
                info!("Connection healthy! Resetting backoff.");
            }
            *delay = 1;
        }
    }

    /// Signals that a reconnection is required.
    pub fn mark_for_reconnect(&self) {
        self.needs_reconnect.store(true, Ordering::SeqCst);
    }

    /// Checks if a reconnection is required.
    pub fn should_reconnect(&self) -> bool {
        self.needs_reconnect.load(Ordering::SeqCst)
    }
}

impl Clone for ConnectionManager {
    fn clone(&self) -> Self {
        Self {
            backoff_secs: self.backoff_secs.clone(),
            needs_reconnect: self.needs_reconnect.clone(),
        }
    }
}
