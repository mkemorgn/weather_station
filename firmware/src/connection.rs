use std::sync::{Arc, Mutex};
use log::{info, warn};

pub struct ConnectionManager {
    backoff_secs: Arc<Mutex<u64>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            backoff_secs: Arc::new(Mutex::new(1)),
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

    /// Fails the current attempt, doubling the backoff time up to 60 seconds.
    pub fn fail(&self) {
        if let Ok(mut delay) = self.backoff_secs.lock() {
            *delay = std::cmp::min(*delay * 2, 60);
            warn!("Connection attempt failed. Next backoff: {}s", *delay);
        }
    }

    /// Resets the backoff to the initial state (1 second).
    pub fn reset(&self) {
        if let Ok(mut delay) = self.backoff_secs.lock() {
            if *delay > 1 {
                info!("Connection healthy! Resetting backoff.");
            }
            *delay = 1;
        }
    }
}

impl Clone for ConnectionManager {
    fn clone(&self) -> Self {
        Self {
            backoff_secs: self.backoff_secs.clone(),
        }
    }
}
