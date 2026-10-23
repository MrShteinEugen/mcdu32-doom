// SPDX-License-Identifier: GPL-3.0-only
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// A one-element mailbox: publishing replaces an obsolete frame, never queues it.
///
/// ```
/// use mcdu_doom::runtime::latest::Latest;
/// let frames = Latest::default();
/// frames.publish(1);
/// frames.publish(2);
/// assert_eq!(frames.take(), Some(2));
/// ```
pub struct Latest<T> {
    value: Mutex<Option<T>>,
    ready: Condvar,
}

impl<T> Default for Latest<T> {
    fn default() -> Self {
        Self {
            value: Mutex::new(None),
            ready: Condvar::new(),
        }
    }
}

impl<T> Latest<T> {
    pub fn publish(&self, value: T) {
        *self.value.lock().expect("frame mailbox poisoned") = Some(value);
        self.ready.notify_one();
    }

    pub fn take(&self) -> Option<T> {
        self.value.lock().expect("frame mailbox poisoned").take()
    }

    pub fn wait(&self, timeout: Duration) -> Option<T> {
        let lock = self.value.lock().expect("frame mailbox poisoned");
        let (mut lock, _) = self
            .ready
            .wait_timeout_while(lock, timeout, |value| value.is_none())
            .expect("frame mailbox poisoned");
        lock.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn latest_replaces_old_frame() {
        let mailbox = Latest::default();
        mailbox.publish(1);
        mailbox.publish(2);
        assert_eq!(mailbox.take(), Some(2));
        assert_eq!(mailbox.take(), None);
    }
}
